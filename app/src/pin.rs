// PIN di sblocco locale. Salvato solo come hash Argon2id nel keychain.

use crate::logdb;
use crate::session;
use crate::secure_store;
use crate::secure_store::err_to_string;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};

const PIN_MIN_LEN: usize = 4;
const PIN_MAX_LEN: usize = 8;
const MAX_ATTEMPTS: u32 = 5;

static FAILED_ATTEMPTS: AtomicU32 = AtomicU32::new(0);

#[derive(Serialize)]
pub struct PinOutcome {
	// "ok" | "wrong" | "locked"
	status: &'static str,
	remaining: u32,
}

#[tauri::command]
pub fn set_pin(pin: String) -> Result<(), String> {
	validate_format(&pin)?;
	let salt = SaltString::generate(&mut OsRng);
	let hash = Argon2::default().hash_password(pin.as_bytes(), &salt).map_err(err_to_string)?.to_string();
	secure_store::set(secure_store::KEY_PIN_HASH, &hash)?;
	FAILED_ATTEMPTS.store(0, Ordering::SeqCst);
	session::set_unlocked(true);
	Ok(())
}

#[tauri::command]
pub fn verify_pin(pin: String) -> Result<PinOutcome, String> {
	let stored = match secure_store::get(secure_store::KEY_PIN_HASH) {
		Some(stored) => stored,
		None => return Err("PIN non impostato".to_string()),
	};
	let parsed = PasswordHash::new(&stored).map_err(err_to_string)?;
	if Argon2::default().verify_password(pin.as_bytes(), &parsed).is_ok() {
		FAILED_ATTEMPTS.store(0, Ordering::SeqCst);
		session::set_unlocked(true);
		return Ok(PinOutcome { status: "ok", remaining: MAX_ATTEMPTS });
	}
	let failed = FAILED_ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1;
	if failed >= MAX_ATTEMPTS {
		// Troppi tentativi: sessione e PIN eliminati, serve un nuovo pair.
		session::set_unlocked(false);
		secure_store::clear_session();
		FAILED_ATTEMPTS.store(0, Ordering::SeqCst);
		let _ = logdb::write("PIN errato troppe volte: sessione eliminata");
		return Ok(PinOutcome { status: "locked", remaining: 0 });
	}
	Ok(PinOutcome { status: "wrong", remaining: MAX_ATTEMPTS - failed })
}

fn validate_format(pin: &str) -> Result<(), String> {
	let length = pin.chars().count();
	if length < PIN_MIN_LEN || length > PIN_MAX_LEN { return Err(format!("Il PIN deve avere da {} a {} cifre", PIN_MIN_LEN, PIN_MAX_LEN)); }
	if !pin.chars().all(|c| c.is_ascii_digit()) { return Err("Il PIN deve contenere solo cifre".to_string()); }
	Ok(())
}