// DB locale essenziale (SQLite) per i log applicativi.
// Tabella Log: logID (incrementale, univoco), Log (testo), datetime (UTC, "YYYY-MM-DD HH:MM:SS").

use crate::secure_store::err_to_string;
use rusqlite::{params, Connection, Row};
use serde::Serialize;
use std::fs;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager};

const DB_FILE: &str = "optigest_offline.db";
const DEFAULT_LIST_LIMIT: u32 = 200;
const MAX_LIST_LIMIT: u32 = 1000;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS Log (
	logID INTEGER PRIMARY KEY AUTOINCREMENT,
	Log TEXT NOT NULL,
	datetime TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
";

static DB: Mutex<Option<Connection>> = Mutex::new(None);

#[derive(Serialize)]
pub struct LogEntry {
	#[serde(rename = "logID")]
	log_id: i64,
	log: String,
	datetime: String,
}

pub fn init(app: &AppHandle) -> Result<(), String> {
	let dir = app.path().app_data_dir().map_err(err_to_string)?;
	fs::create_dir_all(&dir).map_err(err_to_string)?;
	let connection = Connection::open(dir.join(DB_FILE)).map_err(err_to_string)?;
	connection.execute_batch(SCHEMA).map_err(err_to_string)?;
	*lock() = Some(connection);
	Ok(())
}

// Mai dati sensibili nel messaggio (token, PIN, dati paziente).
pub fn write(message: &str) -> Result<(), String> {
	let guard = lock();
	let connection = match guard.as_ref() {
		Some(connection) => connection,
		None => return Err("DB log non inizializzato".to_string()),
	};
	connection.execute("INSERT INTO Log (Log) VALUES (?1)", params![message]).map_err(err_to_string)?;
	Ok(())
}

// Dal più recente al più vecchio.
#[tauri::command]
pub fn list_logs(limit: Option<u32>) -> Result<Vec<LogEntry>, String> {
	let max = limit.unwrap_or(DEFAULT_LIST_LIMIT).min(MAX_LIST_LIMIT);
	let guard = lock();
	let connection = match guard.as_ref() {
		Some(connection) => connection,
		None => return Err("DB log non inizializzato".to_string()),
	};
	let mut statement = connection.prepare("SELECT logID, Log, datetime FROM Log ORDER BY logID DESC LIMIT ?1").map_err(err_to_string)?;
	let rows = statement.query_map(params![max], row_to_entry).map_err(err_to_string)?;
	let mut entries: Vec<LogEntry> = Vec::new();
	for row in rows { entries.push(row.map_err(err_to_string)?); }
	Ok(entries)
}

fn row_to_entry(row: &Row) -> rusqlite::Result<LogEntry> {
	Ok(LogEntry { log_id: row.get(0)?, log: row.get(1)?, datetime: row.get(2)? })
}

fn lock() -> MutexGuard<'static, Option<Connection>> {
	DB.lock().unwrap_or_else(PoisonError::into_inner)
}