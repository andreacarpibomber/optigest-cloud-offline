// Pazienti dal DB locale: ricerca con filtri per colonna, ordinamento e paginazione, e scheda del singolo paziente.
// Tutte le query escludono i pazienti eliminati. I valori cercati passano sempre come parametri;
// l'unica parte di SQL che varia (ORDER BY) viene da una lista fissa, mai dal testo ricevuto.

use crate::db;
use crate::secure_store::err_to_string;
use rusqlite::{named_params, Row};
use serde::{Deserialize, Serialize};

const DEFAULT_PAGE_SIZE: u32 = 10;
const MAX_PAGE_SIZE: u32 = 100;

const FILTER_SQL: &str = "PatientDeleted = 0
	AND (:id = '' OR CAST(PatientID AS TEXT) LIKE '%' || :id || '%' ESCAPE '\\')
	AND (:surname = '' OR PatientSurname LIKE '%' || :surname || '%' ESCAPE '\\')
	AND (:name = '' OR PatientName LIKE '%' || :name || '%' ESCAPE '\\')
	AND (:birthday = '' OR PatientBirthday = :birthday)
	AND (:fiscal_code = '' OR PatientFiscalCode LIKE '%' || :fiscal_code || '%' ESCAPE '\\')";

const LIST_COLUMNS: &str = "PatientID, PatientSurname, PatientName, PatientBirthday, PatientFiscalCode";

const DETAIL_SQL: &str = "SELECT PatientID, PatientName, PatientSurname, PatientBirthday, PatientBirthPlace, PatientBirthProvince,
	PatientGender, PatientFiscalCode, PatientEmail, PatientTelephones, PatientAddress, PatientCity, PatientProvince,
	PatientPostalCode, PatientNation, PatientSendingPracticioner, PatientCurrentPractitioner, PatientTags, PatientAllergies,
	PatientGeneralAnamnesy, PatientOcularAnamnesy, PatientInfo, PatientCreatedOn, PatientLastUpdate
	FROM Patient WHERE PatientID = :id AND PatientDeleted = 0";

#[derive(Deserialize)]
#[serde(default)]
pub struct PatientQuery {
	pub id: String,
	pub surname: String,
	pub name: String,
	// "YYYY-MM-DD" (come il campo data della pagina)
	pub birthday: String,
	pub fiscal_code: String,
	// "id" | "surname" | "name" | "birthday" | "fiscal_code"
	pub sort: String,
	pub desc: bool,
	// da 1
	pub page: u32,
	pub page_size: u32,
}

impl Default for PatientQuery {
	fn default() -> PatientQuery {
		PatientQuery {
			id: String::new(),
			surname: String::new(),
			name: String::new(),
			birthday: String::new(),
			fiscal_code: String::new(),
			sort: "id".to_string(),
			desc: true,
			page: 1,
			page_size: DEFAULT_PAGE_SIZE,
		}
	}
}

#[derive(Serialize)]
pub struct PatientRow {
	id: i64,
	surname: String,
	name: String,
	birthday: Option<String>,
	fiscal_code: String,
}

#[derive(Serialize)]
pub struct PatientPage {
	rows: Vec<PatientRow>,
	total: i64,
	page: u32,
	page_size: u32,
}

#[derive(Serialize)]
pub struct PatientDetail {
	id: i64,
	name: String,
	surname: String,
	birthday: Option<String>,
	birth_place: String,
	birth_province: String,
	gender: String,
	fiscal_code: String,
	email: String,
	telephones: String,
	address: String,
	city: String,
	province: String,
	postal_code: String,
	nation: String,
	sending_practitioner: String,
	current_practitioner: String,
	tags: String,
	allergies: String,
	general_anamnesis: String,
	ocular_anamnesis: String,
	info: String,
	created_on: String,
	last_update: String,
}

pub fn search(query: PatientQuery) -> Result<PatientPage, String> {
	let page = query.page.max(1);
	let page_size = if query.page_size == 0 { DEFAULT_PAGE_SIZE } else { query.page_size.min(MAX_PAGE_SIZE) };
	let offset = (page as i64 - 1) * page_size as i64;
	let id = escape_like(query.id.trim());
	let surname = escape_like(query.surname.trim());
	let name = escape_like(query.name.trim());
	let birthday = query.birthday.trim().to_string();
	let fiscal_code = escape_like(query.fiscal_code.trim());
	let list_sql = format!(
		"SELECT {} FROM Patient WHERE {} ORDER BY {} LIMIT :limit OFFSET :offset",
		LIST_COLUMNS,
		FILTER_SQL,
		order_by(&query.sort, query.desc)
	);
	let count_sql = format!("SELECT COUNT(*) FROM Patient WHERE {}", FILTER_SQL);

	let guard = db::lock();
	let connection = db::get(&guard)?;
	let total: i64 = connection
		.query_row(
			&count_sql,
			named_params! { ":id": id, ":surname": surname, ":name": name, ":birthday": birthday, ":fiscal_code": fiscal_code },
			read_total,
		)
		.map_err(err_to_string)?;
	let mut statement = connection.prepare(&list_sql).map_err(err_to_string)?;
	let found = statement
		.query_map(
			named_params! {
				":id": id, ":surname": surname, ":name": name, ":birthday": birthday, ":fiscal_code": fiscal_code,
				":limit": page_size, ":offset": offset
			},
			row_to_patient,
		)
		.map_err(err_to_string)?;
	let mut rows: Vec<PatientRow> = Vec::new();
	for row in found { rows.push(row.map_err(err_to_string)?); }
	Ok(PatientPage { rows, total, page, page_size })
}

pub fn get(id: i64) -> Result<PatientDetail, String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	match connection.query_row(DETAIL_SQL, named_params! { ":id": id }, row_to_detail) {
		Ok(detail) => Ok(detail),
		Err(rusqlite::Error::QueryReturnedNoRows) => Err("Paziente non trovato".to_string()),
		Err(error) => Err(err_to_string(error)),
	}
}

// Parti fisse e controllate: il testo ricevuto sceglie solo tra queste voci.
// Il tie-break su PatientID rende l'ordine stabile tra pagine.
fn order_by(sort: &str, desc: bool) -> String {
	let column = match sort {
		"surname" => "PatientSurname COLLATE NOCASE",
		"name" => "PatientName COLLATE NOCASE",
		"birthday" => "PatientBirthday",
		"fiscal_code" => "PatientFiscalCode COLLATE NOCASE",
		_ => "PatientID",
	};
	let direction = if desc { "DESC" } else { "ASC" };
	format!("{} {}, PatientID DESC", column, direction)
}

// % e _ cercati dall'utente valgono come caratteri normali (ESCAPE '\').
fn escape_like(text: &str) -> String {
	text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn read_total(row: &Row) -> rusqlite::Result<i64> {
	row.get(0)
}

fn row_to_patient(row: &Row) -> rusqlite::Result<PatientRow> {
	Ok(PatientRow { id: row.get(0)?, surname: row.get(1)?, name: row.get(2)?, birthday: row.get(3)?, fiscal_code: row.get(4)? })
}

fn row_to_detail(row: &Row) -> rusqlite::Result<PatientDetail> {
	Ok(PatientDetail {
		id: row.get(0)?,
		name: row.get(1)?,
		surname: row.get(2)?,
		birthday: row.get(3)?,
		birth_place: row.get(4)?,
		birth_province: row.get(5)?,
		gender: row.get(6)?,
		fiscal_code: row.get(7)?,
		email: row.get(8)?,
		telephones: row.get(9)?,
		address: row.get(10)?,
		city: row.get(11)?,
		province: row.get(12)?,
		postal_code: row.get(13)?,
		nation: row.get(14)?,
		sending_practitioner: row.get(15)?,
		current_practitioner: row.get(16)?,
		tags: row.get(17)?,
		allergies: row.get(18)?,
		general_anamnesis: row.get(19)?,
		ocular_anamnesis: row.get(20)?,
		info: row.get(21)?,
		created_on: row.get(22)?,
		last_update: row.get(23)?,
	})
}