// Licenza: scadenza firmata dal cloud, verificata dall'app.
//
// Il cloud firma (Ed25519) un piccolo documento con la scadenza. Nell'app c'è solo la chiave PUBBLICA (costante qui sotto):
// chi modifica il documento salvato ne invalida la firma e non può firmarne uno nuovo perché la chiave privata sta solo
// sui server (optigestcloudoffline.ini). Il documento contiene anche `offline_until`: oltre quella data l'app vuole
// ricontrollare online. Un orologio portato indietro viene riconosciuto.
//
// Limite noto: chi ricompila/modifica l'eseguibile può saltare il controllo. La difesa vera è lato server
// (la sincronizzazione rifiuta le organizzazioni scadute); qui si rende la manomissione dei dati non banale.

use crate::globals;
use crate::logdb;
use crate::secure_store;
use crate::sync;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// Chiave PUBBLICA di licenza (base64, 32 byte), stampata da scripts/gen_license_key.py. Incollarla qui una volta sola.
const LICENSE_PUBLIC_KEY_B64: &str = "PhmISLDsZ/qahTEkPjEcZob8vhlbKLWURVtYQKiRlVI=";
// Avviso "in scadenza" negli ultimi giorni
const EXPIRING_DAYS: i64 = 30;
// Tolleranza sull'orologio (secondi)
const CLOCK_TOLERANCE_SECS: i64 = 600;
// Scrittura di "ultimo istante visto" nel keychain al massimo una volta al minuto
const SEEN_WRITE_STEP_SECS: i64 = 60;

#[derive(Serialize, Deserialize)]
struct Stored {
	payload: String,
	signature: String,
}

#[derive(Deserialize)]
struct Payload {
	// "YYYY-MM-DD": ultimo giorno di validità (incluso, UTC)
	valid_until: String,
	server_time: i64,
	offline_until: i64,
}

#[derive(Serialize)]
pub struct LicenseStatus {
	// valid | expiring | expired | stale | tampered | missing
	state: &'static str,
	valid_until: Option<String>,
	days_left: Option<i64>,
	// true = l'app non mostra i dati
	blocked: bool,
	message: String,
}

// Dati protetti: l'app non li mostra se la licenza non è valida.
pub fn ensure_ok() -> Result<(), String> {
	let status = status();
	if status.blocked { return Err(format!("LICENSE_BLOCKED: {}", status.message)); }
	Ok(())
}

// Stato calcolato solo dai dati salvati: funziona offline.
pub fn status() -> LicenseStatus {
	if !globals::LICENSE_ENFORCED { return outcome("valid", None, None, false, "Controllo licenza disattivato (build di sviluppo)"); }
	let stored = match secure_store::get(secure_store::KEY_LICENSE).and_then(|text| serde_json::from_str::<Stored>(&text).ok()) {
		Some(stored) => stored,
		None => return outcome("missing", None, None, true, "Licenza non ancora verificata: collegarsi a internet e riaccedere"),
	};
	// Chiave pubblica dell'app non configurata: è un errore di configurazione, non una manomissione.
	if let Err(message) = license_key() { return outcome("tampered", None, None, true, &message); }
	let payload = match verify(&stored) {
		Ok(payload) => payload,
		Err(_) => return outcome("tampered", None, None, true, "Licenza non valida: riaccedere online per rinnovarne la verifica"),
	};
	let now = now_secs();
	let seen = read_seen().max(payload.server_time);
	if now + CLOCK_TOLERANCE_SECS < seen { return outcome("tampered", Some(payload.valid_until), None, true, "Data e ora del computer non coerenti: correggerle e riavviare"); }
	remember_seen(now);

	let end = match day_end_epoch(&payload.valid_until) {
		Some(end) => end,
		None => return outcome("tampered", None, None, true, "Licenza non leggibile: riaccedere online"),
	};
	let days_left = (end - now).div_euclid(86400);
	if now > end { return outcome("expired", Some(payload.valid_until), Some(days_left), true, "Licenza scaduta: contattare EosMedTech per il rinnovo"); }
	if now > payload.offline_until { return outcome("stale", Some(payload.valid_until), Some(days_left), true, "Serve un controllo online della licenza: collegarsi a internet e riaccedere"); }
	if days_left <= EXPIRING_DAYS { return outcome("expiring", Some(payload.valid_until), Some(days_left), false, "Licenza in scadenza"); }
	outcome("valid", Some(payload.valid_until), Some(days_left), false, "Licenza valida")
}

// Chiede al cloud la licenza firmata; se valida la salva. Offline: errore, resta quella salvata.
pub async fn refresh() -> Result<LicenseStatus, String> {
	let (payload, signature) = sync::fetch_license().await?;
	let stored = Stored { payload, signature };
	let parsed = verify(&stored)?;
	let text = serde_json::to_string(&stored).map_err(|error| error.to_string())?;
	secure_store::set(secure_store::KEY_LICENSE, &text)?;
	remember_seen(parsed.server_time.max(now_secs()));
	let _ = logdb::write(&format!("Licenza aggiornata (scade il {})", parsed.valid_until));
	Ok(status())
}

fn verify(stored: &Stored) -> Result<Payload, String> {
	let key = license_key()?;
	let signature = decode_signature(&stored.signature)?;
	key.verify(stored.payload.as_bytes(), &signature).map_err(|_| "Firma della licenza non valida".to_string())?;
	serde_json::from_str(&stored.payload).map_err(|_| "Licenza non leggibile".to_string())
}

// Chiave pubblica di licenza incorporata. Tollera spazi e apici incollati insieme al valore; gli errori dicono cosa non va.
fn license_key() -> Result<VerifyingKey, String> {
	let text = LICENSE_PUBLIC_KEY_B64.trim_matches(|c: char| c.is_whitespace() || c == '\'' || c == '"');
	if text.starts_with("REPLACE_WITH") { return Err("Chiave pubblica di licenza non configurata in license.rs (eseguire scripts/gen_license_key.py)".to_string()); }
	key_from_base64(text, "Chiave pubblica di licenza")
}

fn key_from_base64(text: &str, label: &str) -> Result<VerifyingKey, String> {
	let bytes = STANDARD.decode(text).map_err(|_| format!("{} non è base64 valido ({} caratteri): incollare solo il valore, senza apici", label, text.len()))?;
	let length = bytes.len();
	let array: [u8; 32] = bytes.try_into().map_err(|_| format!("{} di {} byte (attesi 32): è una chiave privata?", label, length))?;
	VerifyingKey::from_bytes(&array).map_err(|_| format!("{} non valida", label))
}

fn decode_signature(text: &str) -> Result<Signature, String> {
	let bytes = STANDARD.decode(text.trim()).map_err(|_| "Firma non valida".to_string())?;
	Signature::from_slice(&bytes).map_err(|_| "Firma non valida".to_string())
}

fn outcome(state: &'static str, valid_until: Option<String>, days_left: Option<i64>, blocked: bool, message: &str) -> LicenseStatus {
	LicenseStatus { state, valid_until, days_left, blocked, message: message.to_string() }
}

fn now_secs() -> i64 {
	SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs() as i64).unwrap_or(0)
}

// Istante più recente osservato (dal cloud o dall'orologio): serve a riconoscere un orologio portato indietro.
fn read_seen() -> i64 {
	secure_store::get(secure_store::KEY_LICENSE_SEEN).and_then(|text| text.parse().ok()).unwrap_or(0)
}

fn remember_seen(instant: i64) {
	if instant < read_seen() + SEEN_WRITE_STEP_SECS { return; }
	let _ = secure_store::set(secure_store::KEY_LICENSE_SEEN, &instant.to_string());
}

// "YYYY-MM-DD" -> secondi Unix della fine di quel giorno (UTC)
fn day_end_epoch(date: &str) -> Option<i64> {
	let parts: Vec<&str> = date.split('-').collect();
	if parts.len() != 3 { return None; }
	let year: i64 = parts[0].parse().ok()?;
	let month: i64 = parts[1].parse().ok()?;
	let day: i64 = parts[2].parse().ok()?;
	if !(1..=12).contains(&month) || !(1..=31).contains(&day) { return None; }
	Some(days_from_civil(year, month, day) * 86400 + 86399)
}

// Giorni dal 1970-01-01 (algoritmo di H. Hinnant)
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
	let y = if month <= 2 { year - 1 } else { year };
	let era = y.div_euclid(400);
	let yoe = y - era * 400;
	let mp = (month + 9) % 12;
	let doy = (153 * mp + 2) / 5 + day - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	era * 146097 + doe - 719468
}