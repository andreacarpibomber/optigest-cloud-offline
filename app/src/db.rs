// DB locale (SQLite). Gli script di schema stanno in app/db/ (un file per tabella) e sono
// incorporati nel binario, nell'ordine di SCHEMA_SCRIPTS.
//
// Traduzione dal DB MySQL di OptiGest Cloud, senza il prefisso organizzazione (_ORG_PREFIX_):
//   date -> TEXT "YYYY-MM-DD", datetime -> TEXT "YYYY-MM-DD HH:MM:SS" (UTC), json -> TEXT con json_valid.
//   Gli ID arrivano dal cloud (nessun AUTOINCREMENT, tranne Log).
//   Gli indici hanno nomi univoci nell'intero DB (in SQLite non sono per tabella).
//
// Installazione: se PRAGMA user_version è inferiore a SCHEMA_VERSION si eseguono gli script
// (idempotenti: CREATE ... IF NOT EXISTS) in una transazione.
// Per modifiche future: aggiornare gli script in db/ e incrementare SCHEMA_VERSION.

use crate::secure_store::err_to_string;
use rusqlite::{Connection, Row};
use std::fs;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager};

const DB_FILE: &str = "optigest_offline.db";
const SCHEMA_VERSION: i64 = 2;

const SCHEMA_SCRIPTS: [&str; 6] = [
	include_str!("../db/Log.sql"),
	include_str!("../db/Patient.sql"),
	include_str!("../db/Visit.sql"),
	include_str!("../db/Doc.sql"),
	include_str!("../db/Event.sql"),
	include_str!("../db/Sync.sql"),
];

static DB: Mutex<Option<Connection>> = Mutex::new(None);

pub fn init(app: &AppHandle) -> Result<(), String> {
	let dir = app.path().app_data_dir().map_err(err_to_string)?;
	fs::create_dir_all(&dir).map_err(err_to_string)?;
	let connection = Connection::open(dir.join(DB_FILE)).map_err(err_to_string)?;
	install(&connection)?;
	*lock() = Some(connection);
	Ok(())
}

pub fn lock() -> MutexGuard<'static, Option<Connection>> {
	DB.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn get<'a>(guard: &'a MutexGuard<'static, Option<Connection>>) -> Result<&'a Connection, String> {
	match guard.as_ref() {
		Some(connection) => Ok(connection),
		None => Err("DB non inizializzato".to_string()),
	}
}

fn install(connection: &Connection) -> Result<(), String> {
	let version: i64 = connection.query_row("PRAGMA user_version", [], read_version).map_err(err_to_string)?;
	if version >= SCHEMA_VERSION { return Ok(()); }
	let transaction = connection.unchecked_transaction().map_err(err_to_string)?;
	for script in SCHEMA_SCRIPTS { transaction.execute_batch(script).map_err(err_to_string)?; }
	transaction.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(err_to_string)?;
	transaction.commit().map_err(err_to_string)
}

fn read_version(row: &Row) -> rusqlite::Result<i64> {
	row.get(0)
}