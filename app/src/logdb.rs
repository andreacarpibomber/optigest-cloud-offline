// Log applicativi nella tabella Log (schema in schema.sql).
// Log: logID (incrementale, univoco), Log (testo), datetime (UTC, "YYYY-MM-DD HH:MM:SS").

use crate::db;
use crate::secure_store::err_to_string;
use rusqlite::{params, Row};
use serde::Serialize;

const DEFAULT_LIST_LIMIT: u32 = 200;
const MAX_LIST_LIMIT: u32 = 1000;

#[derive(Serialize)]
pub struct LogEntry {
	#[serde(rename = "logID")]
	log_id: i64,
	log: String,
	datetime: String,
}

// Mai dati sensibili nel messaggio (token, PIN, dati paziente).
pub fn write(message: &str) -> Result<(), String> {
	let guard = db::lock();
	let connection = db::get(&guard)?;
	connection.execute("INSERT INTO Log (Log) VALUES (?1)", params![message]).map_err(err_to_string)?;
	Ok(())
}

// Dal più recente al più vecchio.
#[tauri::command]
pub fn list_logs(limit: Option<u32>) -> Result<Vec<LogEntry>, String> {
	let max = limit.unwrap_or(DEFAULT_LIST_LIMIT).min(MAX_LIST_LIMIT);
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let mut statement = connection.prepare("SELECT logID, Log, datetime FROM Log ORDER BY logID DESC LIMIT ?1").map_err(err_to_string)?;
	let rows = statement.query_map(params![max], row_to_entry).map_err(err_to_string)?;
	let mut entries: Vec<LogEntry> = Vec::new();
	for row in rows { entries.push(row.map_err(err_to_string)?); }
	Ok(entries)
}

fn row_to_entry(row: &Row) -> rusqlite::Result<LogEntry> {
	Ok(LogEntry { log_id: row.get(0)?, log: row.get(1)?, datetime: row.get(2)? })
}