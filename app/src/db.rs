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
use rusqlite::{Connection, OpenFlags, Row};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
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
static KEY: Mutex<Option<String>> = Mutex::new(None);
static PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(app: &AppHandle, key: &str) -> Result<(), String> {
	if is_open() { return Ok(()); }
	let dir = app.path().app_data_dir().map_err(err_to_string)?;
	fs::create_dir_all(&dir).map_err(err_to_string)?;
	let path = dir.join(DB_FILE);
	let connection = open(&path, key, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
	install(&connection)?;
	*KEY.lock().unwrap_or_else(PoisonError::into_inner) = Some(key.to_string());
	*PATH.lock().unwrap_or_else(PoisonError::into_inner) = Some(path);
	*lock() = Some(connection);
	Ok(())
}

pub fn is_open() -> bool {
	lock().is_some()
}

pub fn close() {
	*lock() = None;
	*KEY.lock().unwrap_or_else(PoisonError::into_inner) = None;
	*PATH.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

pub fn delete_plaintext(app: &AppHandle) -> Result<(), String> {
	let dir = app.path().app_data_dir().map_err(err_to_string)?;
	let path = dir.join(DB_FILE);
	let mut header = [0u8; 16];
	fs::File::open(&path).and_then(|mut file| file.read_exact(&mut header)).map_err(err_to_string)?;
	if &header != b"SQLite format 3\0" { return Err("Il database non risulta non cifrato".to_string()); }
	close();
	for suffix in ["-wal", "-shm", "-journal", ""] {
		let file = dir.join(format!("{}{}", DB_FILE, suffix));
		if file.exists() { fs::remove_file(file).map_err(err_to_string)?; }
	}
	Ok(())
}

pub fn path() -> Result<PathBuf, String> {
	PATH.lock().unwrap_or_else(PoisonError::into_inner).clone().ok_or("DB non inizializzato".to_string())
}

pub fn open_read_only(path: &Path) -> Result<Connection, String> {
	let key = KEY.lock().unwrap_or_else(PoisonError::into_inner).clone().ok_or("Chiave DB non disponibile".to_string())?;
	open(path, &key, OpenFlags::SQLITE_OPEN_READ_ONLY)
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

fn open(path: &Path, key: &str, flags: OpenFlags) -> Result<Connection, String> {
	if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) { return Err("Chiave DB non valida".to_string()); }
	let connection = Connection::open_with_flags(path, flags).map_err(err_to_string)?;
	connection.execute_batch(&format!("PRAGMA key = \"x'{}'\";", key)).map_err(err_to_string)?;
	let _: String = connection.query_row("PRAGMA cipher_version", [], |row| row.get(0)).map_err(|_| "SQLCipher non disponibile".to_string())?;
	connection.query_row("SELECT count(*) FROM sqlite_master", [], read_version).map_err(|_| "Database non cifrato oppure chiave non valida".to_string())?;
	Ok(connection)
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
