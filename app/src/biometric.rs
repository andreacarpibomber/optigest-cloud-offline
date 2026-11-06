// Sblocco con impronta (Touch ID su macOS, Windows Hello su Windows) al posto del PIN.
// Il PIN resta sempre: serve per il primo sblocco, per attivare l'impronta e come alternativa.
// L'impronta non memorizza nulla in app: il sistema operativo risponde solo sì/no.

use crate::logdb;
use crate::secure_store;
use crate::session;
use serde::Serialize;

const REASON: &str = "Sblocca OptiGest Cloud Offline";

#[derive(Serialize)]
pub struct BiometricStatus {
	// il computer ha un lettore configurato
	available: bool,
	// l'utente l'ha attivato per questa associazione
	enabled: bool,
}

pub fn status() -> BiometricStatus {
	BiometricStatus { available: platform::is_available(), enabled: is_enabled() }
}

fn is_enabled() -> bool {
	secure_store::get(secure_store::KEY_BIOMETRIC).as_deref() == Some("1")
}

// Attivazione (app sbloccata): chiede subito l'impronta, così si sa che funziona.
pub async fn set_enabled(enabled: bool) -> Result<(), String> {
	session::require_unlocked()?;
	if !enabled {
		secure_store::delete(secure_store::KEY_BIOMETRIC);
		let _ = logdb::write("Sblocco con impronta disattivato");
		return Ok(());
	}
	if !platform::is_available() { return Err("Impronta non disponibile su questo computer".to_string()); }
	if !authenticate().await? { return Err("Impronta non riconosciuta".to_string()); }
	secure_store::set(secure_store::KEY_BIOMETRIC, "1")?;
	let _ = logdb::write("Sblocco con impronta attivato");
	Ok(())
}

// Sblocco dalla schermata del PIN. false = annullato o non riconosciuto: si ripiega sul PIN.
pub async fn unlock() -> Result<bool, String> {
	if !is_enabled() || !platform::is_available() { return Ok(false); }
	let token = secure_store::get(secure_store::KEY_TOKEN).unwrap_or_default();
	if token.is_empty() || secure_store::get(secure_store::KEY_PIN_HASH).is_none() { return Ok(false); }
	if !authenticate().await? { return Ok(false); }
	session::set_unlocked(true);
	Ok(true)
}

async fn authenticate() -> Result<bool, String> {
	tauri::async_runtime::spawn_blocking(|| platform::authenticate(REASON))
		.await
		.map_err(|error| error.to_string())?
}

#[cfg(target_os = "macos")]
mod platform {
	use block2::RcBlock;
	use objc2::runtime::Bool;
	use objc2_foundation::{NSError, NSString};
	use objc2_local_authentication::{LAContext, LAPolicy};
	use std::sync::mpsc;
	use std::time::Duration;

	// Solo biometria (Touch ID): senza lettore o senza impronte registrate non è disponibile.
	const POLICY: LAPolicy = LAPolicy::DeviceOwnerAuthenticationWithBiometrics;
	const PROMPT_TIMEOUT_SECS: u64 = 120;

	pub fn is_available() -> bool {
		let context = unsafe { LAContext::new() };
		unsafe { context.canEvaluatePolicy_error(POLICY) }.is_ok()
	}

	pub fn authenticate(reason: &str) -> Result<bool, String> {
		let context = unsafe { LAContext::new() };
		let (sender, receiver) = mpsc::channel::<bool>();
		let reply = RcBlock::new(move |success: Bool, _error: *mut NSError| {
			let _ = sender.send(success.as_bool());
		});
		let text = NSString::from_str(reason);
		unsafe { context.evaluatePolicy_localizedReason_reply(POLICY, &text, &reply) };
		// Il contesto deve restare vivo finché arriva la risposta.
		let outcome = receiver.recv_timeout(Duration::from_secs(PROMPT_TIMEOUT_SECS)).unwrap_or(false);
		drop(context);
		Ok(outcome)
	}
}

#[cfg(target_os = "windows")]
mod platform {
	use windows::core::HSTRING;
	use windows::Security::Credentials::UI::{UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability};

	pub fn is_available() -> bool {
		match UserConsentVerifier::CheckAvailabilityAsync().and_then(|operation| operation.get()) {
			Ok(availability) => availability == UserConsentVerifierAvailability::Available,
			Err(_) => false,
		}
	}

	pub fn authenticate(reason: &str) -> Result<bool, String> {
		let result = UserConsentVerifier::RequestVerificationAsync(&HSTRING::from(reason))
			.and_then(|operation| operation.get())
			.map_err(|error| error.to_string())?;
		Ok(result == UserConsentVerificationResult::Verified)
	}
}

// Linux e altri: nessun lettore standard, resta solo il PIN.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
	pub fn is_available() -> bool { false }
	pub fn authenticate(_reason: &str) -> Result<bool, String> { Ok(false) }
}