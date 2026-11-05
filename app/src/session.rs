// Flusso dello schema su carta:
//   sessione salvata?  NO -> "pair"
//                      SI -> PIN presente?  SI -> "unlock"
//                                           NO -> "set_pin"
// Dopo lo sblocco la UI chiama connect_upstream (validazione token su OptiGest Cloud).

use crate::globals;
use crate::logdb;
use crate::secure_store;
use crate::secure_store::err_to_string;
use serde::Serialize;
use std::time::Duration;

const UPSTREAM_TIMEOUT_SECS: u64 = 8;

#[derive(Serialize)]
pub struct UpstreamInfo {
	// true se il cloud ha risposto (200 o 401)
	online: bool,
	// false solo se il cloud ha rifiutato il token
	valid: bool,
	name: String,
}

#[tauri::command]
pub fn boot_state() -> &'static str {
	let token = secure_store::get(secure_store::KEY_TOKEN).unwrap_or_default();
	if token.is_empty() { return "pair"; }
	if secure_store::get(secure_store::KEY_PIN_HASH).is_none() { return "set_pin"; }
	"unlock"
}

#[tauri::command]
pub async fn connect_upstream() -> Result<UpstreamInfo, String> {
	let token = secure_store::get(secure_store::KEY_TOKEN).unwrap_or_default();
	let cached_name = secure_store::get(secure_store::KEY_NAME).unwrap_or_default();
	if token.is_empty() { return Ok(UpstreamInfo { online: false, valid: false, name: String::new() }); }

	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(UPSTREAM_TIMEOUT_SECS))
		.build()
		.map_err(err_to_string)?;
	let response = match client.get(globals::test_login_url()).query(&[("tkn", token.as_str())]).send().await {
		Ok(response) => response,
		// Offline: si lavora con quanto in cache, il token non viene toccato.
		Err(_) => return Ok(UpstreamInfo { online: false, valid: true, name: cached_name }),
	};

	let status = response.status().as_u16();
	if status == 401 {
		secure_store::clear_session();
		let _ = logdb::write("Token rifiutato dal cloud (401): sessione eliminata");
		return Ok(UpstreamInfo { online: true, valid: false, name: String::new() });
	}
	if status != 200 {
		// 403 = manutenzione, 5xx = problemi server: non si butta la sessione.
		return Ok(UpstreamInfo { online: false, valid: true, name: cached_name });
	}

	let body = response.text().await.unwrap_or_default();
	let name = extract_name(&body).unwrap_or(cached_name);
	if !name.is_empty() { let _ = secure_store::set(secure_store::KEY_NAME, &name); }
	Ok(UpstreamInfo { online: true, valid: true, name })
}

#[tauri::command]
pub fn logout() {
	secure_store::clear_session();
	let _ = logdb::write("Logout");
}

// TODO: da allineare al contratto reale di testLogin.php (oggi si assume un JSON con "name").
fn extract_name(body: &str) -> Option<String> {
	let value: serde_json::Value = serde_json::from_str(body).ok()?;
	let name = value.get("name")?.as_str()?.trim().to_string();
	if name.is_empty() { return None; }
	Some(name)
}