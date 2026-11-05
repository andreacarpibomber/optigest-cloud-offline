// Dati utente/studio ricevuti dal login (testLogin.php), con cache nel keychain per l'uso offline.
// Risposta attesa: {"success": true, "user": {"UserEmail", "UserID", "UserName", "UserSurname", "UserTitle"}}.
// Il nome studio non è (ancora) nella risposta: si cerca in "user" e nella radice sotto le chiavi di ORG_KEYS.
// Le chiavi si confrontano senza distinguere maiuscole/minuscole, quindi vanno scritte minuscole qui.

use serde::{Deserialize, Serialize};
use serde_json::Value;

const FIRST_NAME_KEYS: [&str; 3] = ["username", "firstname", "name"];
const SURNAME_KEYS: [&str; 3] = ["usersurname", "surname", "lastname"];
const FULL_NAME_KEYS: [&str; 2] = ["fullname", "displayname"];
const ORG_KEYS: [&str; 7] = ["orgname", "org", "organization", "organizationname", "practicename", "practice", "studio"];
const TITLE_KEYS: [&str; 3] = ["usertitle", "title", "prefix"];
const SECRET_KEY_PARTS: [&str; 4] = ["tkn", "token", "pass", "pwd"];
const MAX_RAW_CHARS: usize = 2000;

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
#[serde(default)]
pub struct Profile {
	pub name: String,
	pub org: String,
	pub title: String,
}

impl Profile {
	pub fn from_body(body: &str) -> Profile {
		let value: Value = match serde_json::from_str(body) {
			Ok(value) => value,
			Err(_) => return Profile::default(),
		};
		let user = match value.get("user") {
			Some(user) if user.is_object() => user,
			_ => &value,
		};
		let mut org = find_string(user, &ORG_KEYS);
		if org.is_empty() { org = find_string(&value, &ORG_KEYS); }
		Profile { name: full_name(user), org: org, title: find_string(user, &TITLE_KEYS) }
	}

	// Cache nel keychain: JSON. Il vecchio formato (solo nome in chiaro) viene letto come nome.
	pub fn from_cache(text: &str) -> Profile {
		match serde_json::from_str::<Profile>(text) {
			Ok(profile) => profile,
			Err(_) => Profile { name: text.trim().to_string(), ..Profile::default() },
		}
	}

	pub fn to_cache(&self) -> String {
		serde_json::to_string(self).unwrap_or_default()
	}

	pub fn is_empty(&self) -> bool {
		self.name.is_empty() && self.org.is_empty() && self.title.is_empty()
	}

	// I campi mancanti nella risposta si prendono dalla cache.
	pub fn with_fallback(self, fallback: &Profile) -> Profile {
		Profile {
			name: pick(self.name, &fallback.name),
			org: pick(self.org, &fallback.org),
			title: pick(self.title, &fallback.title),
		}
	}
}

// Risposta del cloud per il debug a schermo: JSON formattato con i campi segreti oscurati,
// oppure il testo troncato se non è JSON.
pub fn redacted(body: &str) -> String {
	let mut value: Value = match serde_json::from_str(body) {
		Ok(value) => value,
		Err(_) => return body.chars().take(MAX_RAW_CHARS).collect(),
	};
	redact(&mut value);
	serde_json::to_string_pretty(&value).unwrap_or_default()
}

fn full_name(user: &Value) -> String {
	let first = find_string(user, &FIRST_NAME_KEYS);
	let surname = find_string(user, &SURNAME_KEYS);
	if first.is_empty() && surname.is_empty() { return find_string(user, &FULL_NAME_KEYS); }
	format!("{} {}", first, surname).trim().to_string()
}

// Primo valore testuale non vuoto, nell'ordine delle chiavi richieste (chiavi dell'oggetto confrontate in minuscolo).
fn find_string(value: &Value, keys: &[&str]) -> String {
	let map = match value.as_object() {
		Some(map) => map,
		None => return String::new(),
	};
	for key in keys {
		for (name, child) in map {
			if name.to_lowercase() != *key { continue; }
			if let Some(text) = child.as_str() {
				let trimmed = text.trim();
				if !trimmed.is_empty() { return trimmed.to_string(); }
			}
		}
	}
	String::new()
}

fn pick(value: String, fallback: &str) -> String {
	if value.is_empty() { return fallback.to_string(); }
	value
}

fn redact(value: &mut Value) {
	match value {
		Value::Object(map) => {
			for (key, child) in map.iter_mut() {
				if is_secret_key(key) { *child = Value::String("***".to_string()); } else { redact(child); }
			}
		}
		Value::Array(items) => {
			for item in items.iter_mut() { redact(item); }
		}
		_ => {}
	}
}

fn is_secret_key(key: &str) -> bool {
	let lower = key.to_lowercase();
	for part in SECRET_KEY_PARTS {
		if lower.contains(part) { return true; }
	}
	false
}