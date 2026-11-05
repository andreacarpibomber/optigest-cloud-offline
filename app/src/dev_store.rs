// SOLO SVILUPPO (compilato solo nelle build debug): store dei segreti su file JSON in chiaro,
// al posto del keychain, per evitare le richieste di password a ogni compilazione.
// Si attiva con OPTIGEST_DEV_STORE=file (vedi secure_store::init). Mai per dati reali.

use crate::secure_store::err_to_string;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

static PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn activate(path: PathBuf) {
	*lock() = Some(path);
}

pub fn active() -> bool {
	lock().is_some()
}

pub fn read(key: &str) -> Option<String> {
	let path = current()?;
	let map = load(&path);
	map.get(key)?.as_str().map(str::to_string)
}

pub fn write(key: &str, value: &str) -> Result<(), String> {
	let path = current().ok_or("Dev store non attivo".to_string())?;
	let mut map = load(&path);
	map.insert(key.to_string(), Value::String(value.to_string()));
	save(&path, &map)
}

pub fn remove(key: &str) {
	let path = match current() {
		Some(path) => path,
		None => return,
	};
	let mut map = load(&path);
	map.remove(key);
	let _ = save(&path, &map);
}

fn lock() -> MutexGuard<'static, Option<PathBuf>> {
	PATH.lock().unwrap_or_else(PoisonError::into_inner)
}

fn current() -> Option<PathBuf> {
	lock().clone()
}

fn load(path: &Path) -> Map<String, Value> {
	let text = match fs::read_to_string(path) {
		Ok(text) => text,
		Err(_) => return Map::new(),
	};
	match serde_json::from_str::<Map<String, Value>>(&text) {
		Ok(map) => map,
		Err(_) => Map::new(),
	}
}

fn save(path: &Path, map: &Map<String, Value>) -> Result<(), String> {
	let text = serde_json::to_string_pretty(map).map_err(err_to_string)?;
	fs::write(path, text).map_err(err_to_string)?;
	restrict(path);
	Ok(())
}

#[cfg(unix)]
fn restrict(path: &Path) {
	use std::os::unix::fs::PermissionsExt;
	let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_path: &Path) {}