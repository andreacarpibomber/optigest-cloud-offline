// Backup del DB locale con rotazione (default: ultime 3 copie).
//
// Regole:
//   - il backup parte solo se il DB è integro (PRAGMA integrity_check) e non c'è una sincronizzazione
//     in corso o incompleta (uno scarico completo a metà darebbe una copia parziale)
//   - la copia (VACUUM INTO) viene verificata (integrità e numero di pazienti/visite) prima di diventare un backup
//   - la rotazione elimina le copie vecchie solo dopo un backup riuscito: se qualcosa va storto restano tutte
//   - ogni errore è salvato in backups/last_error.txt e mostrato all'utente fino al prossimo backup riuscito
//   - automatico: dopo una sincronizzazione riuscita, al massimo una volta ogni 24 ore
// Le copie sono file SQLite normali (contengono dati dei pazienti: stessa cartella e stesse protezioni del DB).

use crate::config;
use crate::db;
use crate::logdb;
use crate::secure_store::err_to_string;
use crate::sync;
use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use tauri::{AppHandle, Manager};

pub const DEFAULT_KEEP: u32 = 3;
pub const MIN_KEEP: u32 = 1;
pub const MAX_KEEP: u32 = 10;
const MIN_INTERVAL_SECS: i64 = 24 * 3600;

const BACKUP_DIR: &str = "backups";
const FILE_PREFIX: &str = "optigest_offline-";
const FILE_EXT: &str = ".db";
const ERROR_FILE: &str = "last_error.txt";

static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Serialize)]
pub struct BackupInfo {
	name: String,
	// "YYYY-MM-DD HH:MM:SS" (UTC)
	created_on: String,
	size_bytes: u64,
}

#[derive(Serialize)]
pub struct BackupStatus {
	keep: u32,
	min_keep: u32,
	max_keep: u32,
	backups: Vec<BackupInfo>,
	// ultimo errore, finché non c'è un backup riuscito
	last_error: Option<String>,
}

#[derive(Serialize)]
pub struct BackupOutcome {
	// true = backup creato; false = non eseguito (vedi message)
	done: bool,
	message: String,
}

pub fn init(app: &AppHandle) -> Result<(), String> {
	let dir = app.path().app_data_dir().map_err(err_to_string)?.join(BACKUP_DIR);
	fs::create_dir_all(&dir).map_err(err_to_string)?;
	*DIR.lock().unwrap_or_else(PoisonError::into_inner) = Some(dir);
	Ok(())
}

pub fn status() -> Result<BackupStatus, String> {
	Ok(BackupStatus {
		keep: config::backup_keep(),
		min_keep: MIN_KEEP,
		max_keep: MAX_KEEP,
		backups: list()?,
		last_error: read_error(),
	})
}

// Numero di copie da tenere (retention). Applicato subito: le copie in più, le più vecchie, vengono eliminate.
pub fn set_keep(keep: u32) -> Result<BackupStatus, String> {
	if keep < MIN_KEEP || keep > MAX_KEEP { return Err(format!("Il numero di copie deve essere tra {} e {}", MIN_KEEP, MAX_KEEP)); }
	config::set_backup_keep(keep)?;
	rotate(keep)?;
	status()
}

// force = true: backup manuale (ignora l'intervallo di 24 ore, non i controlli di integrità).
pub fn run(force: bool) -> Result<BackupOutcome, String> {
	let result = run_checked(force);
	match &result {
		Ok(outcome) if outcome.done => {
			clear_error();
			let _ = logdb::write("Backup DB locale eseguito");
		}
		Err(message) => {
			store_error(message);
			let _ = logdb::write(&format!("Backup DB locale non riuscito: {}", message));
		}
		Ok(_) => {}
	}
	result
}

fn run_checked(force: bool) -> Result<BackupOutcome, String> {
	if let Some(reason) = sync::blocks_backup()? { return Ok(skipped(&reason)); }
	let dir = backup_dir()?;
	if !force && newest_age_secs()?.map_or(false, |age| age < MIN_INTERVAL_SECS) { return Ok(skipped("Backup recente già presente")); }

	let (final_path, temp_path, live_counts);
	{
		let guard = db::lock();
		let connection = db::get(&guard)?;
		check_integrity(connection).map_err(|reason| format!("{}. Nessun backup eseguito: le copie esistenti sono state mantenute. Il DB locale è una copia dei dati del cloud: chiudi l'app, svuotalo con scripts/reset_db.py e sincronizza di nuovo (oppure ripristina un backup precedente)", reason))?;
		let stamp: String = connection.query_row("SELECT strftime('%Y%m%d-%H%M%S','now')", [], |row| row.get(0)).map_err(err_to_string)?;
		final_path = dir.join(format!("{}{}{}", FILE_PREFIX, stamp, FILE_EXT));
		temp_path = dir.join(format!("{}{}.tmp", FILE_PREFIX, stamp));
		if final_path.exists() { return Ok(skipped("Backup già presente per questo secondo")); }
		let _ = fs::remove_file(&temp_path);
		live_counts = counts(connection)?;
		if let Err(error) = connection.execute("VACUUM INTO ?1", params![temp_path.to_string_lossy().to_string()]) {
			let _ = fs::remove_file(&temp_path);
			return Err(format!("Copia del DB non riuscita: {}", error));
		}
	}

	if let Err(reason) = verify_copy(&temp_path, live_counts) {
		let _ = fs::remove_file(&temp_path);
		return Err(format!("La copia creata non è valida ({}): backup scartato, copie esistenti mantenute", reason));
	}
	if let Err(error) = fs::rename(&temp_path, &final_path) {
		let _ = fs::remove_file(&temp_path);
		return Err(format!("Salvataggio del backup non riuscito: {}", error));
	}
	rotate(config::backup_keep())?;
	Ok(BackupOutcome { done: true, message: "Backup eseguito".to_string() })
}

fn skipped(message: &str) -> BackupOutcome {
	BackupOutcome { done: false, message: message.to_string() }
}

// Integrità completa del DB: la risposta attesa è la sola riga "ok".
fn check_integrity(connection: &Connection) -> Result<(), String> {
	let mut statement = connection.prepare("PRAGMA integrity_check").map_err(err_to_string)?;
	let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(err_to_string)?;
	let mut problems: Vec<String> = Vec::new();
	for row in rows { problems.push(row.map_err(err_to_string)?); }
	if problems.len() == 1 && problems[0] == "ok" { return Ok(()); }
	let shown: Vec<String> = problems.into_iter().take(3).collect();
	Err(format!("Il DB locale non è integro ({})", shown.join("; ")))
}

fn counts(connection: &Connection) -> Result<(i64, i64), String> {
	let patients: i64 = connection.query_row("SELECT COUNT(*) FROM Patient", [], |row| row.get(0)).map_err(err_to_string)?;
	let visits: i64 = connection.query_row("SELECT COUNT(*) FROM Visit", [], |row| row.get(0)).map_err(err_to_string)?;
	Ok((patients, visits))
}

// La copia si apre da sola, è integra e ha gli stessi numeri del DB in uso.
fn verify_copy(path: &PathBuf, expected: (i64, i64)) -> Result<(), String> {
	let copy = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err_to_string)?;
	check_integrity(&copy)?;
	if counts(&copy)? != expected { return Err("numero di pazienti o visite diverso dal DB in uso".to_string()); }
	Ok(())
}

// Tiene le `keep` copie più recenti (il nome contiene data e ora, quindi l'ordine alfabetico è cronologico).
fn rotate(keep: u32) -> Result<(), String> {
	let dir = backup_dir()?;
	let backups = list()?;
	for old in backups.iter().skip(keep as usize) {
		fs::remove_file(dir.join(&old.name)).map_err(|error| format!("Eliminazione del vecchio backup {} non riuscita: {}", old.name, error))?;
	}
	Ok(())
}

// Dal più recente al più vecchio.
fn list() -> Result<Vec<BackupInfo>, String> {
	let dir = backup_dir()?;
	let mut backups: Vec<BackupInfo> = Vec::new();
	for entry in fs::read_dir(&dir).map_err(err_to_string)? {
		let entry = entry.map_err(err_to_string)?;
		let name = entry.file_name().to_string_lossy().to_string();
		let created_on = match name_to_datetime(&name) {
			Some(created_on) => created_on,
			None => continue,
		};
		let size_bytes = entry.metadata().map_err(err_to_string)?.len();
		backups.push(BackupInfo { name, created_on, size_bytes });
	}
	backups.sort_by(|a, b| b.name.cmp(&a.name));
	Ok(backups)
}

// "optigest_offline-20261106-234500.db" -> "2026-11-06 23:45:00"
fn name_to_datetime(name: &str) -> Option<String> {
	let stamp = name.strip_prefix(FILE_PREFIX)?.strip_suffix(FILE_EXT)?;
	if stamp.len() != 15 || stamp.as_bytes()[8] != b'-' || !stamp.chars().enumerate().all(|(index, c)| index == 8 || c.is_ascii_digit()) { return None; }
	Some(format!("{}-{}-{} {}:{}:{}", &stamp[0..4], &stamp[4..6], &stamp[6..8], &stamp[9..11], &stamp[11..13], &stamp[13..15]))
}

fn newest_age_secs() -> Result<Option<i64>, String> {
	let newest = match list()?.into_iter().next() {
		Some(newest) => newest,
		None => return Ok(None),
	};
	let guard = db::lock();
	let connection = db::get(&guard)?;
	let age: Option<i64> = connection
		.query_row("SELECT CAST(strftime('%s','now') AS INTEGER) - CAST(strftime('%s', ?1) AS INTEGER)", params![newest.created_on], |row| row.get(0))
		.map_err(err_to_string)?;
	Ok(age)
}

fn backup_dir() -> Result<PathBuf, String> {
	match DIR.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
		Some(dir) => Ok(dir.clone()),
		None => Err("Cartella dei backup non inizializzata".to_string()),
	}
}

// L'errore sta in un file accanto ai backup (non nel DB: potrebbe essere proprio il DB il problema).
fn store_error(message: &str) {
	if let Ok(dir) = backup_dir() { let _ = fs::write(dir.join(ERROR_FILE), message); }
}

fn clear_error() {
	if let Ok(dir) = backup_dir() { let _ = fs::remove_file(dir.join(ERROR_FILE)); }
}

fn read_error() -> Option<String> {
	let dir = backup_dir().ok()?;
	let text = fs::read_to_string(dir.join(ERROR_FILE)).ok()?;
	if text.trim().is_empty() { return None; }
	Some(text)
}