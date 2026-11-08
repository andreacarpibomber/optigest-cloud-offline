// Storage nel keychain del sistema (Credential Manager / Keychain / Secret Service).
// Equivalente sicuro dell'UserDefaults usato nell'app Swift.

use keyring::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::AppHandle;

const SERVICE: &str = "com.eosmedtech.optigest.offline";

// Tutti i segreti stanno in UN solo elemento del keychain (JSON), letto una volta per esecuzione e tenuto in memoria.
// Un elemento per segreto voleva dire una richiesta di password del sistema per ciascuno.
const BLOB_ACCOUNT: &str = "all_secrets";

// Un elemento per chiave: formato precedente, migrato nell'elemento unico alla prima lettura.
const LEGACY_KEYS: [&str; 6] = ["login_token", "pin_hash", "display_name", "biometric_enabled", "license_signed", "license_seen"];

static SECRETS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

// Solo build debug: OPTIGEST_DEV_STORE=file usa un file in chiaro invece del keychain (vedi dev_store.rs).
#[cfg(debug_assertions)]
const DEV_STORE_ENV: &str = "OPTIGEST_DEV_STORE";
#[cfg(debug_assertions)]
const DEV_STORE_FILE: &str = "dev_secure_store.json";

pub const KEY_TOKEN: &str = "login_token";
pub const KEY_PIN_HASH: &str = "pin_hash";
pub const KEY_NAME: &str = "display_name";
pub const KEY_BIOMETRIC: &str = "biometric_enabled";
pub const KEY_LICENSE: &str = "license_signed";
pub const KEY_LICENSE_SEEN: &str = "license_seen";

pub fn init(app: &AppHandle) -> Result<(), String> {
	#[cfg(debug_assertions)]
	{
		use tauri::Manager;
		if std::env::var(DEV_STORE_ENV).as_deref() == Ok("file") {
			let dir = app.path().app_data_dir().map_err(err_to_string)?;
			std::fs::create_dir_all(&dir).map_err(err_to_string)?;
			let path = dir.join(DEV_STORE_FILE);
			eprintln!("ATTENZIONE: segreti in chiaro in {} (solo sviluppo, niente keychain)", path.display());
			crate::dev_store::activate(path);
		}
	}
	let _ = app;
	Ok(())
}

pub fn get(key: &str) -> Option<String> {
	#[cfg(debug_assertions)]
	{
		if crate::dev_store::active() { return crate::dev_store::read(key); }
	}
	let mut guard = lock();
	if load_into(&mut guard).is_err() { return None; }
	guard.as_ref().and_then(|secrets| secrets.get(key).cloned())
}

pub fn set(key: &str, value: &str) -> Result<(), String> {
	#[cfg(debug_assertions)]
	{
		if crate::dev_store::active() { return crate::dev_store::write(key, value); }
	}
	let mut guard = lock();
	load_into(&mut guard)?;
	let secrets = guard.as_mut().ok_or("Segreti non disponibili".to_string())?;
	secrets.insert(key.to_string(), value.to_string());
	persist(secrets)
}

pub fn delete(key: &str) {
	remove_keys(&[key]);
}

// Equivalente di removeLoginToken() + reset PIN: riporta l'app allo stato "da associare".
pub fn clear_session() {
	remove_keys(&[KEY_TOKEN, KEY_PIN_HASH, KEY_NAME, KEY_BIOMETRIC]);
}

fn remove_keys(keys: &[&str]) {
	#[cfg(debug_assertions)]
	{
		if crate::dev_store::active() {
			for key in keys { crate::dev_store::remove(key); }
			return;
		}
	}
	let mut guard = lock();
	if load_into(&mut guard).is_err() { return; }
	if let Some(secrets) = guard.as_mut() {
		for key in keys { secrets.remove(*key); }
		let _ = persist(secrets);
	}
}

fn lock() -> MutexGuard<'static, Option<HashMap<String, String>>> {
	SECRETS.lock().unwrap_or_else(PoisonError::into_inner)
}

// Carica i segreti dal keychain se non sono già in memoria. Se il keychain nega l'accesso resta un errore
// (non si scambia mai "accesso negato" per "nessun segreto": si rischierebbe di sovrascrivere quelli veri).
fn load_into(guard: &mut MutexGuard<'static, Option<HashMap<String, String>>>) -> Result<(), String> {
	if guard.is_some() { return Ok(()); }
	let entry = Entry::new(SERVICE, BLOB_ACCOUNT).map_err(err_to_string)?;
	let secrets = match entry.get_password() {
		Ok(text) => serde_json::from_str::<HashMap<String, String>>(&text).map_err(err_to_string)?,
		Err(keyring::Error::NoEntry) => migrate_legacy()?,
		Err(error) => return Err(error.to_string()),
	};
	**guard = Some(secrets);
	Ok(())
}

// Prima esecuzione dopo l'aggiornamento: i segreti dei vecchi elementi passano nell'elemento unico.
fn migrate_legacy() -> Result<HashMap<String, String>, String> {
	let mut secrets: HashMap<String, String> = HashMap::new();
	for key in LEGACY_KEYS {
		let entry = Entry::new(SERVICE, key).map_err(err_to_string)?;
		match entry.get_password() {
			Ok(value) => {
				secrets.insert(key.to_string(), value);
			}
			Err(keyring::Error::NoEntry) => {}
			Err(error) => return Err(error.to_string()),
		}
	}
	if secrets.is_empty() { return Ok(secrets); }
	persist(&secrets)?;
	for key in LEGACY_KEYS {
		if let Ok(entry) = Entry::new(SERVICE, key) { let _ = entry.delete_credential(); }
	}
	Ok(secrets)
}

fn persist(secrets: &HashMap<String, String>) -> Result<(), String> {
	let text = serde_json::to_string(secrets).map_err(err_to_string)?;
	let entry = Entry::new(SERVICE, BLOB_ACCOUNT).map_err(err_to_string)?;
	entry.set_password(&text).map_err(err_to_string)
}

pub fn err_to_string<E: ToString>(error: E) -> String {
	error.to_string()
}