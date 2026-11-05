// Log in console delle chiamate al cloud: richiesta e risposta, solo nelle build di debug e se globals::HTTP_LOG_ENABLED è true.
// Il token è mascherato. Le risposte di sync contengono dati dei pazienti: nelle build release non si stampa nulla.

#[cfg(debug_assertions)]
use crate::globals;

#[cfg(debug_assertions)]
const MAX_BODY_CHARS: usize = 3000;

#[cfg(debug_assertions)]
pub fn request(label: &str, url: &str, params: &[(&str, String)]) {
	if !globals::HTTP_LOG_ENABLED { return; }
	let shown: Vec<String> = params.iter().map(|(key, value)| format!("{}={}", key, mask(key, value))).collect();
	let joiner = if url.contains('?') { "&" } else { "?" };
	eprintln!("[cloud] -> {} GET {}{}{}", label, url, joiner, shown.join("&"));
}

#[cfg(debug_assertions)]
pub fn response(label: &str, status: u16, body: &str) {
	if !globals::HTTP_LOG_ENABLED { return; }
	let total = body.chars().count();
	let text: String = body.chars().take(MAX_BODY_CHARS).collect();
	if total > MAX_BODY_CHARS {
		eprintln!("[cloud] <- {} HTTP {} ({} caratteri, mostrati {}):\n{}", label, status, total, MAX_BODY_CHARS, text);
	} else {
		eprintln!("[cloud] <- {} HTTP {}:\n{}", label, status, text);
	}
}

#[cfg(debug_assertions)]
pub fn failure(label: &str, error: &str) {
	if !globals::HTTP_LOG_ENABLED { return; }
	eprintln!("[cloud] !! {} {}", label, error);
}

#[cfg(debug_assertions)]
fn mask(key: &str, value: &str) -> String {
	if key != "tkn" { return value.to_string(); }
	let head: String = value.chars().take(4).collect();
	format!("{}***", head)
}

#[cfg(not(debug_assertions))]
pub fn request(_label: &str, _url: &str, _params: &[(&str, String)]) {}

#[cfg(not(debug_assertions))]
pub fn response(_label: &str, _status: u16, _body: &str) {}

#[cfg(not(debug_assertions))]
pub fn failure(_label: &str, _error: &str) {}