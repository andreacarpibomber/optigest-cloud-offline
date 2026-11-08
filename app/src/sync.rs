// Sincronizzazione cloud -> DB locale (sola lettura), protocollo v2 su /api/syncData.php.
//
// Richieste: GET con "tkn" (loginTkn, come le altre API), "device" (id installazione) e "fn":
//   handshake  protocol, app_version        -> { success, protocol, epochs: {patient, visit, event}, max_batch }
//   pull       table, limit, rewind_s,
//              cursor_updated_on, cursor_id -> { success, rows: [{colonna: valore}], next_cursor: {updated_on, id}, has_more }
// Errori: 401 token non accettato (sessione intatta), 426 app troppo vecchia, altri codici = errore del cloud.
//
// Un epoch per tabella. Primo accesso o epoch di una tabella cambiato (es. backup ripristinato sul cloud):
// full sync di QUELLA tabella = pull dal cursore zero. Se si riscaricano i pazienti si riscaricano anche le visite.
// A fine full sync le righe locali che il cloud non ha mandato vengono eliminate (nulla viene eliminato prima).
// Dopo: pull incrementale dal cursore, riletto con una finestra di sicurezza (rewind_s) sulla prima pagina.
// Ogni pagina si applica in una transazione insieme al suo cursore: un'interruzione riparte dall'ultima pagina.

use crate::backup;
use crate::db;
use crate::globals;
use crate::httplog;
use crate::logdb;
use crate::secure_store::{self, err_to_string};
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const SESSION_EXPIRED: &str = "SESSION_EXPIRED";

const PROTOCOL_VERSION: u32 = 2;
const REQUEST_TIMEOUT_SECS: u64 = 60;
const MAX_BATCH: u32 = 2000;
const SAFETY_WINDOW_SECS: u32 = 60;

const STATE_LAST_SYNC: &str = "last_sync_on";
const STATE_DEVICE_ID: &str = "device_id";
const FULL_PENDING: &str = "pending";
const FULL_DONE: &str = "done";

static RUNNING: AtomicBool = AtomicBool::new(false);

// Colonne uguali nel DB locale e nelle righe che manda il cloud.
struct SyncTable {
	name: &'static str,
	sql_table: &'static str,
	id_column: &'static str,
	columns: &'static [&'static str],
}

// Pazienti prima delle visite; gli appuntamenti (calendario) per ultimi.
const TABLES: [SyncTable; 3] = [
	SyncTable {
		name: "patient",
		sql_table: "Patient",
		id_column: "PatientID",
		columns: &[
			"PatientID", "PatientName", "PatientSurname", "PatientBirthday", "PatientBirthPlace", "PatientBirthProvince",
			"PatientGender", "PatientFiscalCode", "PatientSendingPracticioner", "PatientCurrentPractitioner", "PatientEmail",
			"PatientTags", "PatientAllergies", "PatientGeneralAnamnesy", "PatientOcularAnamnesy", "PatientAddress",
			"PatientCity", "PatientProvince", "PatientPostalCode", "PatientNation", "PatientInfo", "PatientOptions",
			"PatientTelephones", "PatientLastConsentDate", "PatientCreatedOn", "PatientLastUpdate", "PatientDeleted",
		],
	},
	SyncTable {
		name: "visit",
		sql_table: "Visit",
		id_column: "VisitID",
		columns: &["VisitID", "VPatientID", "VDate", "VCause", "VDiagnosis", "VData", "VChecksum", "VLastUpdate", "VDeleted"],
	},
	SyncTable {
		name: "event",
		sql_table: "Event",
		id_column: "EventID",
		columns: &["EventID", "EventOwnerID", "EventLastEditorID", "EventOrgID", "EventTitle", "EventStart", "EventEnd", "EventData", "EventLastEdit", "EventDeleted"],
	},
];

#[derive(Deserialize)]
struct Handshake {
	protocol: u32,
	epochs: HashMap<String, String>,
	#[serde(default)]
	max_batch: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Cursor {
	updated_on: String,
	id: i64,
}

#[derive(Deserialize)]
struct PullResponse {
	rows: Vec<Map<String, Value>>,
	next_cursor: Option<Cursor>,
	has_more: bool,
}

#[derive(Serialize, Clone)]
pub struct Progress {
	pub table: &'static str,
	pub rows: u64,
}

#[derive(Serialize, Default)]
pub struct SyncReport {
	full: bool,
	patients: u64,
	visits: u64,
	events: u64,
	removed: u64,
}

#[derive(Serialize)]
pub struct SyncStatus {
	last_sync_on: Option<String>,
	full_pending: bool,
	running: bool,
	// secondi da attendere prima di poter sincronizzare di nuovo (0 = subito)
	wait_secs: i64,
}

pub fn status() -> Result<SyncStatus, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let last_sync_on = get_state(connection, STATE_LAST_SYNC)?;
	let full_pending = any_full_pending(connection)?;
	let wait_secs = wait_seconds(connection, full_pending)?;
	Ok(SyncStatus { last_sync_on, full_pending, running: RUNNING.load(Ordering::SeqCst), wait_secs })
}

// Licenza firmata dal cloud: (documento, firma). La verifica della firma la fa license.rs.
pub async fn fetch_license() -> Result<(String, String), String> {
	let token = secure_store::get(secure_store::KEY_TOKEN).unwrap_or_default();
	if token.is_empty() { return Err(SESSION_EXPIRED.to_string()); }
	let device_id = device_id()?;
	let client = reqwest::Client::builder().timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)).cookie_store(true).build().map_err(err_to_string)?;
	let response: LicenseResponse = call(&client, &token, &device_id, &[("fn", "license".to_string())]).await?;
	Ok((response.license.payload, response.license.signature))
}

#[derive(Deserialize)]
struct LicenseResponse {
	license: LicenseDocument,
}

#[derive(Deserialize)]
struct LicenseDocument {
	payload: String,
	signature: String,
}

// Motivo per cui ora non conviene fare un backup del DB (sync in corso o scarico completo a metà).
pub fn blocks_backup() -> Result<Option<String>, String> {
	let status = status()?;
	if status.running { return Ok(Some("Sincronizzazione in corso: backup rimandato".to_string())); }
	if status.full_pending { return Ok(Some("Scarico completo non terminato: backup rimandato".to_string())); }
	Ok(None)
}

// Secondi mancanti al prossimo sync consentito, in base all'ultimo riuscito.
fn wait_seconds(connection: &Connection, full_pending: bool) -> Result<i64, String> {
	if full_pending { return Ok(0); }
	let elapsed: Option<i64> = connection
		.query_row(
			"SELECT CAST(strftime('%s','now') AS INTEGER) - CAST(strftime('%s', Value) AS INTEGER) FROM SyncState WHERE Name = ?1",
			params![STATE_LAST_SYNC],
			|row| row.get(0),
		)
		.optional()
		.map_err(err_to_string)?
		.flatten();
	match elapsed {
		Some(seconds) if seconds >= 0 && seconds < globals::SYNC_MIN_INTERVAL_SECS => Ok(globals::SYNC_MIN_INTERVAL_SECS - seconds),
		_ => Ok(0),
	}
}

pub async fn run(on_progress: &(dyn Fn(Progress) + Send + Sync)) -> Result<SyncReport, String> {
	let wait = status()?.wait_secs;
	if wait > 0 { return Err(format!("Sincronizzazione già eseguita da poco: riprova tra {}", describe_wait(wait))); }
	if RUNNING.swap(true, Ordering::SeqCst) { return Err("Sincronizzazione già in corso".to_string()); }
	let result = run_all(on_progress).await;
	RUNNING.store(false, Ordering::SeqCst);
	// Dopo una sincronizzazione riuscita: backup del DB (se integro, al massimo uno al giorno). Gli errori restano visibili all'utente.
	if result.is_ok() { let _ = tauri::async_runtime::spawn_blocking(|| backup::run(false)).await; }
	log_result(&result);
	result
}

async fn run_all(on_progress: &(dyn Fn(Progress) + Send + Sync)) -> Result<SyncReport, String> {
	let token = secure_store::get(secure_store::KEY_TOKEN).unwrap_or_default();
	if token.is_empty() { return Err(SESSION_EXPIRED.to_string()); }
	let device_id = device_id()?;
	// cookie_store: il cloud apre una sessione al primo login col token e la riconosce nelle richieste successive.
	let client = reqwest::Client::builder().timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)).cookie_store(true).build().map_err(err_to_string)?;

	let handshake: Handshake = call(&client, &token, &device_id, &[
		("fn", "handshake".to_string()),
		("protocol", PROTOCOL_VERSION.to_string()),
		("app_version", env!("CARGO_PKG_VERSION").to_string()),
	]).await?;
	if handshake.protocol != PROTOCOL_VERSION { return Err("Protocollo di sincronizzazione non compatibile: aggiornare l'app".to_string()); }
	let batch = if handshake.max_batch == 0 { MAX_BATCH } else { handshake.max_batch.min(MAX_BATCH) };

	let fulls = prepare_full_sync(&handshake.epochs)?;
	let mut report = SyncReport { full: fulls.iter().any(|full| *full), ..SyncReport::default() };
	for (index, table) in TABLES.iter().enumerate() {
		let count = sync_table(&client, &token, &device_id, table, batch, fulls[index], on_progress).await?;
		match table.name {
			"patient" => report.patients = count,
			"visit" => report.visits = count,
			_ => report.events = count,
		}
	}
	report.removed = finish_full_sync(&fulls)?;
	mark_synced()?;
	Ok(report)
}

async fn sync_table(
	client: &reqwest::Client,
	token: &str,
	device_id: &str,
	table: &SyncTable,
	batch: u32,
	full: bool,
	on_progress: &(dyn Fn(Progress) + Send + Sync),
) -> Result<u64, String> {
	let mut cursor = load_cursor(table)?;
	let mut first_page = true;
	let mut total: u64 = 0;
	loop {
		// La finestra di sicurezza vale solo per la prima pagina di un ciclo incrementale.
		let rewind = if first_page && !full && cursor.is_some() { SAFETY_WINDOW_SECS } else { 0 };
		first_page = false;
		let mut query = vec![
			("fn", "pull".to_string()),
			("table", table.name.to_string()),
			("limit", batch.to_string()),
			("rewind_s", rewind.to_string()),
		];
		if let Some(position) = &cursor {
			query.push(("cursor_updated_on", position.updated_on.clone()));
			query.push(("cursor_id", position.id.to_string()));
		}
		let page: PullResponse = call(client, token, device_id, &query).await?;
		if page.has_more && (page.rows.is_empty() || page.next_cursor.is_none()) { return Err("Risposta del cloud non valida (pagina vuota con altre pagine)".to_string()); }
		total += apply_page(table, &page, full)?;
		on_progress(Progress { table: table.name, rows: total });
		if !page.has_more { return Ok(total); }
		cursor = page.next_cursor;
	}
}

// Etichetta per il log: fn e tabella (es. "pull patient").
fn describe_call(query: &[(&str, String)]) -> String {
	let mut parts: Vec<String> = Vec::new();
	for (key, value) in query {
		if *key == "fn" || *key == "table" { parts.push(value.clone()); }
	}
	format!("sync {}", parts.join(" "))
}

async fn call<T: serde::de::DeserializeOwned>(
	client: &reqwest::Client,
	token: &str,
	device_id: &str,
	query: &[(&str, String)],
) -> Result<T, String> {
	let mut params: Vec<(&str, String)> = query.to_vec();
	params.push(("tkn", token.to_string()));
	params.push(("device", device_id.to_string()));
	let label = describe_call(query);
	httplog::request(&label, &globals::sync_url(), &params);
	let sent = client.get(globals::sync_url()).query(&params).header("Accept", "application/json").send().await;
	let response = match sent {
		Ok(response) => response,
		Err(error) => {
			httplog::failure(&label, &error.to_string());
			return Err(describe_send_error(&error));
		}
	};
	let status = response.status().as_u16();
	let body = response.text().await.map_err(err_to_string)?;
	httplog::response(&label, status, &body);
	// 401 = token non accettato dalla sincronizzazione (es. strumento non collegato). Non è la sessione di login
	// scaduta (quella la verifica il login): la sessione non si tocca, altrimenti si finirebbe in un ciclo di login.
	if status == 426 { return Err("App troppo vecchia per sincronizzare: aggiornare".to_string()); }
	if status != 200 { return Err(describe_http_error(status, &body)); }
	let value: Value = serde_json::from_str(&body).map_err(describe_parse_error)?;
	// Come le altre API: anche con HTTP 200 può esserci { success: false, error } (es. troppe richieste).
	if value.get("success").and_then(Value::as_bool) == Some(false) { return Err(describe_cloud_error(&value)); }
	serde_json::from_value::<T>(value).map_err(describe_parse_error)
}

// Applica una pagina e il suo cursore in un'unica transazione. Ritorna le righe applicate.
fn apply_page(table: &SyncTable, page: &PullResponse, full: bool) -> Result<u64, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let transaction = connection.unchecked_transaction().map_err(err_to_string)?;
	let upsert = upsert_sql(table);
	let defaults = required_defaults(&transaction, table)?;
	{
		let mut statement = transaction.prepare(&upsert).map_err(err_to_string)?;
		let mut seen = transaction.prepare("INSERT OR IGNORE INTO SyncSeen (TableName, ID) VALUES (?1, ?2)").map_err(err_to_string)?;
		for row in &page.rows {
			let values: Vec<SqlValue> = table.columns.iter().map(|column| to_sql_or_default(row.get(*column), defaults.get(*column))).collect();
			statement.execute(params_from_iter(values.iter())).map_err(err_to_string)?;
			if full { seen.execute(params![table.name, id_of(table, row)?]).map_err(err_to_string)?; }
		}
	}
	if let Some(position) = &page.next_cursor {
		let text = serde_json::to_string(position).map_err(err_to_string)?;
		set_state(&transaction, &cursor_key(table), &text)?;
	}
	transaction.commit().map_err(err_to_string)?;
	Ok(page.rows.len() as u64)
}

// Per ogni tabella decide se questo ciclo è una full sync. Epoch nuovo o diverso: la tabella riparte da zero
// (cursore e ID visti azzerati). Le visite dipendono dai pazienti: se i pazienti ripartono, ripartono anche loro.
fn prepare_full_sync(epochs: &HashMap<String, String>) -> Result<Vec<bool>, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let transaction = connection.unchecked_transaction().map_err(err_to_string)?;
	let mut fulls: Vec<bool> = Vec::new();
	let mut patient_reset = false;
	for table in TABLES.iter() {
		let epoch = epochs.get(table.name).ok_or(format!("Epoch mancante nella risposta del cloud ({})", table.name))?;
		let stored_epoch = get_state(&transaction, &epoch_key(table))?;
		let pending = get_state(&transaction, &full_key(table))?.as_deref() == Some(FULL_PENDING);
		let changed = stored_epoch.as_deref() != Some(epoch.as_str());
		let reset = changed || (table.name == "visit" && patient_reset);
		if reset {
			transaction.execute("DELETE FROM SyncState WHERE Name = ?1", params![cursor_key(table)]).map_err(err_to_string)?;
			transaction.execute("DELETE FROM SyncSeen WHERE TableName = ?1", params![table.name]).map_err(err_to_string)?;
			set_state(&transaction, &epoch_key(table), epoch)?;
			set_state(&transaction, &full_key(table), FULL_PENDING)?;
		}
		if table.name == "patient" { patient_reset = reset; }
		fulls.push(reset || pending);
	}
	transaction.commit().map_err(err_to_string)?;
	Ok(fulls)
}

// Fine scarico completo: via le righe locali che il cloud non ha mandato (dati di prova, record spariti).
// Vale solo per le tabelle che hanno fatto la full sync in questo ciclo.
fn finish_full_sync(fulls: &[bool]) -> Result<u64, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let transaction = connection.unchecked_transaction().map_err(err_to_string)?;
	let mut removed: u64 = 0;
	for (index, table) in TABLES.iter().enumerate() {
		if !fulls[index] { continue; }
		let sql = format!(
			"DELETE FROM {} WHERE {} NOT IN (SELECT ID FROM SyncSeen WHERE TableName = ?1)",
			table.sql_table, table.id_column
		);
		removed += transaction.execute(&sql, params![table.name]).map_err(err_to_string)? as u64;
		transaction.execute("DELETE FROM SyncSeen WHERE TableName = ?1", params![table.name]).map_err(err_to_string)?;
		set_state(&transaction, &full_key(table), FULL_DONE)?;
	}
	transaction.commit().map_err(err_to_string)?;
	Ok(removed)
}

// Almeno una tabella ha uno scarico completo da fare: non terminato, oppure mai iniziato (nessuna epoca salvata,
// per esempio al primo avvio dopo un aggiornamento). In questo caso l'attesa tra una sync e l'altra non vale.
fn any_full_pending(connection: &Connection) -> Result<bool, String> {
	for table in TABLES.iter() {
		if get_state(connection, &full_key(table))?.as_deref() == Some(FULL_PENDING) { return Ok(true); }
		if get_state(connection, &epoch_key(table))?.is_none() { return Ok(true); }
	}
	Ok(false)
}

fn describe_wait(seconds: i64) -> String {
	if seconds >= 60 { return format!("{} min", (seconds + 59) / 60); }
	format!("{} s", seconds)
}

fn mark_synced() -> Result<(), String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	connection
		.execute(
			"INSERT INTO SyncState (Name, Value) VALUES (?1, datetime('now')) ON CONFLICT(Name) DO UPDATE SET Value = excluded.Value",
			params![STATE_LAST_SYNC],
		)
		.map_err(err_to_string)?;
	Ok(())
}

// Identificativo casuale di questa installazione (per il log lato cloud), creato al primo uso.
fn device_id() -> Result<String, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	if let Some(existing) = get_state(connection, STATE_DEVICE_ID)? { return Ok(existing); }
	let created: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], read_text).map_err(err_to_string)?;
	set_state(connection, STATE_DEVICE_ID, &created)?;
	Ok(created)
}

fn load_cursor(table: &SyncTable) -> Result<Option<Cursor>, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let text = match get_state(connection, &cursor_key(table))? {
		Some(text) => text,
		None => return Ok(None),
	};
	match serde_json::from_str::<Cursor>(&text) {
		Ok(cursor) => Ok(Some(cursor)),
		Err(_) => Ok(None),
	}
}

fn get_state(connection: &Connection, name: &str) -> Result<Option<String>, String> {
	connection
		.query_row("SELECT Value FROM SyncState WHERE Name = ?1", params![name], read_text)
		.optional()
		.map_err(err_to_string)
}

fn set_state(connection: &Connection, name: &str, value: &str) -> Result<(), String> {
	connection
		.execute(
			"INSERT INTO SyncState (Name, Value) VALUES (?1, ?2) ON CONFLICT(Name) DO UPDATE SET Value = excluded.Value",
			params![name, value],
		)
		.map_err(err_to_string)?;
	Ok(())
}

fn log_result(result: &Result<SyncReport, String>) {
	let message = match result {
		Ok(report) => format!(
			"Sincronizzazione {} completata: {} pazienti, {} visite, {} appuntamenti, {} righe locali rimosse",
			if report.full { "completa" } else { "incrementale" },
			report.patients,
			report.visits,
			report.events,
			report.removed
		),
		Err(error) => format!("Sincronizzazione non riuscita: {}", error),
	};
	let _ = logdb::write(&message);
}

// INSERT ... ON CONFLICT: aggiorna tutte le colonne tranne l'ID. Nomi di colonna solo da TABLES, mai dal cloud.
fn upsert_sql(table: &SyncTable) -> String {
	let names = table.columns.join(", ");
	let marks: Vec<String> = (1..=table.columns.len()).map(|n| format!("?{}", n)).collect();
	let updates: Vec<String> = table.columns.iter().filter(|c| **c != table.id_column).map(|c| format!("{} = excluded.{}", c, c)).collect();
	format!(
		"INSERT INTO {} ({}) VALUES ({}) ON CONFLICT({}) DO UPDATE SET {}",
		table.sql_table,
		names,
		marks.join(", "),
		table.id_column,
		updates.join(", ")
	)
}

fn cursor_key(table: &SyncTable) -> String {
	format!("cursor.{}", table.name)
}

fn epoch_key(table: &SyncTable) -> String {
	format!("epoch.{}", table.name)
}

fn full_key(table: &SyncTable) -> String {
	format!("full_sync.{}", table.name)
}

fn id_of(table: &SyncTable, row: &Map<String, Value>) -> Result<i64, String> {
	match row.get(table.id_column) {
		Some(Value::Number(number)) => number.as_i64().ok_or("ID non valido nella risposta del cloud".to_string()),
		Some(Value::String(text)) => text.trim().parse::<i64>().map_err(err_to_string),
		_ => Err("Riga senza ID nella risposta del cloud".to_string()),
	}
}

// JSON -> valore SQLite. Oggetti e array (colonne JSON) diventano testo.
// Colonne NOT NULL del DB locale (esclusa la chiave) con il valore da usare se il cloud manda NULL:
// 0 per gli interi, "{}" per i dati JSON delle visite, testo vuoto per il resto.
fn required_defaults(connection: &Connection, table: &SyncTable) -> Result<HashMap<String, SqlValue>, String> {
	let mut statement = connection.prepare(&format!("SELECT name, type, \"notnull\", pk FROM pragma_table_info('{}')", table.sql_table)).map_err(err_to_string)?;
	let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))).map_err(err_to_string)?;
	let mut defaults: HashMap<String, SqlValue> = HashMap::new();
	for item in rows {
		let (name, kind, not_null, primary) = item.map_err(err_to_string)?;
		if not_null == 0 || primary != 0 { continue; }
		let value = if kind.eq_ignore_ascii_case("INTEGER") { SqlValue::Integer(0) } else if name == "VData" { SqlValue::Text("{}".to_string()) } else { SqlValue::Text(String::new()) };
		defaults.insert(name, value);
	}
	Ok(defaults)
}

fn to_sql_or_default(value: Option<&Value>, default: Option<&SqlValue>) -> SqlValue {
	match to_sql(value) {
		SqlValue::Null => default.cloned().unwrap_or(SqlValue::Null),
		other => other,
	}
}

fn to_sql(value: Option<&Value>) -> SqlValue {
	match value {
		None | Some(Value::Null) => SqlValue::Null,
		Some(Value::Bool(flag)) => SqlValue::Integer(*flag as i64),
		Some(Value::Number(number)) => match number.as_i64() {
			Some(integer) => SqlValue::Integer(integer),
			None => SqlValue::Real(number.as_f64().unwrap_or(0.0)),
		},
		Some(Value::String(text)) => SqlValue::Text(text.clone()),
		Some(other) => SqlValue::Text(other.to_string()),
	}
}

fn describe_send_error(error: &reqwest::Error) -> String {
	if error.is_connect() || error.is_timeout() { return "Cloud non raggiungibile".to_string(); }
	err_to_string(error)
}

fn describe_cloud_error(value: &Value) -> String {
	match value.get("error").and_then(Value::as_str) {
		Some(text) => format!("Cloud: {}", text),
		None => "Cloud: richiesta rifiutata".to_string(),
	}
}

// Errore HTTP: se il corpo è JSON con "error" lo si mostra insieme al codice.
fn describe_http_error(status: u16, body: &str) -> String {
	let text = match serde_json::from_str::<Value>(body) {
		Ok(value) => value.get("error").and_then(Value::as_str).map(str::to_string),
		Err(_) => None,
	};
	match text {
		Some(text) => format!("Cloud: {} (HTTP {})", text, status),
		None => format!("Cloud: errore HTTP {}", status),
	}
}

fn describe_parse_error(error: serde_json::Error) -> String {
	format!("Risposta del cloud non valida: {}", error)
}

fn read_text(row: &rusqlite::Row) -> rusqlite::Result<String> {
	row.get(0)
}