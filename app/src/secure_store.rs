// Storage nel keychain del sistema (Credential Manager / Keychain / Secret Service).
// Equivalente sicuro dell'UserDefaults usato nell'app Swift.

use keyring::Entry;
use tauri::AppHandle;

const SERVICE: &str = "com.eosmedtech.optigest.offline";

// Solo build debug: OPTIGEST_DEV_STORE=file usa un file in chiaro invece del keychain (vedi dev_store.rs).
#[cfg(debug_assertions)]
const DEV_STORE_ENV: &str = "OPTIGEST_DEV_STORE";
#[cfg(debug_assertions)]
const DEV_STORE_FILE: &str = "dev_secure_store.json";

pub const KEY_TOKEN: &str = "login_token";
pub const KEY_PIN_HASH: &str = "pin_hash";
pub const KEY_NAME: &str = "display_name";
pub const KEY_BIOMETRIC: &str = "biometric_enabled";

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
	let entry = match Entry::new(SERVICE, key) {
		Ok(entry) => entry,
		Err(_) => return None,
	};
	match entry.get_password() {
		Ok(value) => Some(value),
		Err(_) => None,
	}
}

pub fn set(key: &str, value: &str) -> Result<(), String> {
	#[cfg(debug_assertions)]
	{
		if crate::dev_store::active() { return crate::dev_store::write(key, value); }
	}
	let entry = Entry::new(SERVICE, key).map_err(err_to_string)?;
	entry.set_password(value).map_err(err_to_string)
}

pub fn delete(key: &str) {
	#[cfg(debug_assertions)]
	{
		if crate::dev_store::active() { crate::dev_store::remove(key); return; }
	}
	let entry = match Entry::new(SERVICE, key) {
		Ok(entry) => entry,
		Err(_) => return,
	};
	let _ = entry.delete_credential();
}

// Equivalente di removeLoginToken() + reset PIN: riporta l'app allo stato "da associare".
pub fn clear_session() {
	delete(KEY_TOKEN);
	delete(KEY_PIN_HASH);
	delete(KEY_NAME);
	delete(KEY_BIOMETRIC);
}

pub fn err_to_string<E: ToString>(error: E) -> String {
	error.to_string()
}