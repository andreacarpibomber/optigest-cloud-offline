// Storage nel keychain del sistema (Credential Manager / Keychain / Secret Service).
// Equivalente sicuro dell'UserDefaults usato nell'app Swift.

use keyring::Entry;

const SERVICE: &str = "com.eosmedtech.optigest.offline";

pub const KEY_TOKEN: &str = "login_token";
pub const KEY_PIN_HASH: &str = "pin_hash";
pub const KEY_NAME: &str = "display_name";
pub const KEY_HOST: &str = "preferred_host";

pub fn get(key: &str) -> Option<String> {
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
	let entry = Entry::new(SERVICE, key).map_err(err_to_string)?;
	entry.set_password(value).map_err(err_to_string)
}

pub fn delete(key: &str) {
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
}

pub fn err_to_string<E: ToString>(error: E) -> String {
	error.to_string()
}
