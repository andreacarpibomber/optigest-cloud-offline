// PIN di sblocco locale. Salvato solo come hash Argon2id nel keychain.

use crate::db;
use crate::logdb;
use crate::session;
use crate::secure_store;
use crate::secure_store::err_to_string;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305, Nonce};
use serde::Serialize;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Manager};

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

pub fn set_pin(app: &AppHandle, pin: String, database_key: &str) -> Result<(), String> {
	validate_format(&pin)?;
	write_database_key(app, &pin, database_key)?;
	db::init(app, database_key)?;
	if let Err(error) = store_pin(&pin) { db::close(); return Err(error); }
	Ok(())
}

fn store_pin(pin: &str) -> Result<(), String> {
	let salt = SaltString::generate(&mut OsRng);
	let hash = Argon2::default().hash_password(pin.as_bytes(), &salt).map_err(err_to_string)?.to_string();
	secure_store::set(secure_store::KEY_PIN_HASH, &hash)?;
	FAILED_ATTEMPTS.store(0, Ordering::SeqCst);
	session::set_unlocked(true);
	Ok(())
}

pub fn verify_pin(app: &AppHandle, pin: String) -> Result<PinOutcome, String> {
	let stored = match secure_store::get(secure_store::KEY_PIN_HASH) {
		Some(stored) => stored,
		None => return Err("PIN non impostato".to_string()),
	};
	let parsed = PasswordHash::new(&stored).map_err(err_to_string)?;
	if Argon2::default().verify_password(pin.as_bytes(), &parsed).is_ok() {
		let database_key = read_database_key(app, &pin)?;
		db::init(app, &database_key)?;
		FAILED_ATTEMPTS.store(0, Ordering::SeqCst);
		session::set_unlocked(true);
		return Ok(PinOutcome { status: "ok", remaining: MAX_ATTEMPTS });
	}
	let failed = FAILED_ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1;
	if failed >= MAX_ATTEMPTS {
		// Troppi tentativi: sessione e PIN eliminati, serve un nuovo pair.
		session::set_unlocked(false);
		let _ = logdb::write("PIN errato troppe volte: sessione eliminata");
		db::close();
		secure_store::clear_session();
		FAILED_ATTEMPTS.store(0, Ordering::SeqCst);
		return Ok(PinOutcome { status: "locked", remaining: 0 });
	}
	Ok(PinOutcome { status: "wrong", remaining: MAX_ATTEMPTS - failed })
}

// Cambio PIN (app sbloccata): chiede il PIN attuale; gli errori contano come tentativi sbagliati.
pub fn change_pin(app: &AppHandle, current: String, new_pin: String) -> Result<PinOutcome, String> {
	validate_format(&new_pin)?;
	let outcome = verify_pin(app, current.clone())?;
	if outcome.status != "ok" { return Ok(outcome); }
	let database_key = read_database_key(app, &current)?;
	write_database_key(app, &new_pin, &database_key)?;
	store_pin(&new_pin)?;
	let _ = logdb::write("PIN modificato");
	Ok(outcome)
}

pub fn read_database_key(app: &AppHandle, pin: &str) -> Result<String, String> {
	let data = fs::read(app.path().app_data_dir().map_err(err_to_string)?.join("db_key.enc")).map_err(err_to_string)?;
	if data.len() < 30 || data[0] != 1 { return Err("File della chiave non valido".to_string()); }
	let mut wrapping_key = [0u8; 32];
	Argon2::default().hash_password_into(pin.as_bytes(), &data[1..17], &mut wrapping_key).map_err(err_to_string)?;
	let decrypted = ChaCha20Poly1305::new((&wrapping_key).into()).decrypt(Nonce::from_slice(&data[17..29]), &data[29..]);
	wrapping_key.fill(0);
	String::from_utf8(decrypted.map_err(|_| "PIN o file della chiave non valido".to_string())?).map_err(err_to_string)
}

// Formato: versione + salt Argon2 (16 byte) + nonce ChaCha20 (12 byte) + chiave autenticata e cifrata.
fn write_database_key(app: &AppHandle, pin: &str, database_key: &str) -> Result<(), String> {
	let directory = app.path().app_data_dir().map_err(err_to_string)?;
	fs::create_dir_all(&directory).map_err(err_to_string)?;
	let mut salt = [0u8; 16];
	let mut nonce = [0u8; 12];
	OsRng.fill_bytes(&mut salt);
	OsRng.fill_bytes(&mut nonce);
	let mut wrapping_key = [0u8; 32];
	Argon2::default().hash_password_into(pin.as_bytes(), &salt, &mut wrapping_key).map_err(err_to_string)?;
	let encrypted = ChaCha20Poly1305::new((&wrapping_key).into()).encrypt(Nonce::from_slice(&nonce), database_key.as_bytes()).map_err(err_to_string)?;
	wrapping_key.fill(0);
	let mut data = vec![1];
	data.extend_from_slice(&salt);
	data.extend_from_slice(&nonce);
	data.extend_from_slice(&encrypted);
	fs::write(directory.join("db_key.enc"), data).map_err(err_to_string)
}

fn validate_format(pin: &str) -> Result<(), String> {
	let length = pin.chars().count();
	if length < PIN_MIN_LEN || length > PIN_MAX_LEN { return Err(format!("Il PIN deve avere da {} a {} cifre", PIN_MIN_LEN, PIN_MAX_LEN)); }
	if !pin.chars().all(|c| c.is_ascii_digit()) { return Err("Il PIN deve contenere solo cifre".to_string()); }
	Ok(())
}
