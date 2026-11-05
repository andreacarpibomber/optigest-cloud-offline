// Flusso dello schema su carta:
//   sessione salvata?  NO -> "pair"
//                      SI -> PIN presente?  SI -> "unlock"
//                                           NO -> "set_pin"
// Dopo lo sblocco la UI chiama connect_upstream (validazione token su OptiGest Cloud).

use crate::globals;
use crate::logdb;
use crate::profile::{self, Profile};
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
	org: String,
	title: String,
	// risposta del cloud con i segreti oscurati, per valutare a schermo cosa arriva dal login
	raw: String,
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
	let cached = Profile::from_cache(&secure_store::get(secure_store::KEY_NAME).unwrap_or_default());
	if token.is_empty() { return Ok(info(false, false, Profile::default(), String::new())); }

	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(UPSTREAM_TIMEOUT_SECS))
		.build()
		.map_err(err_to_string)?;
	let response = match client.get(globals::test_login_url()).query(&[("tkn", token.as_str())]).send().await {
		Ok(response) => response,
		// Offline: si lavora con quanto in cache, il token non viene toccato.
		Err(_) => return Ok(info(false, true, cached, String::new())),
	};

	let status = response.status().as_u16();
	if status == 401 {
		secure_store::clear_session();
		let _ = logdb::write("Token rifiutato dal cloud (401): sessione eliminata");
		return Ok(info(true, false, Profile::default(), String::new()));
	}
	if status != 200 {
		// 403 = manutenzione, 5xx = problemi server: non si butta la sessione.
		return Ok(info(false, true, cached, String::new()));
	}

	let body = response.text().await.unwrap_or_default();
	let received = Profile::from_body(&body).with_fallback(&cached);
	if !received.is_empty() && received != cached { let _ = secure_store::set(secure_store::KEY_NAME, &received.to_cache()); }
	Ok(info(true, true, received, profile::redacted(&body)))
}

#[tauri::command]
pub fn logout() {
	secure_store::clear_session();
	let _ = logdb::write("Logout");
}

fn info(online: bool, valid: bool, profile: Profile, raw: String) -> UpstreamInfo {
	UpstreamInfo { online, valid, name: profile.name, org: profile.org, title: profile.title, raw }
}