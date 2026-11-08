// Sincronizzazione automatica in background, mentre l'app è aperta (anche nascosta nella barra di sistema).
//
// Due regole, controllate ogni minuto:
//   Notturna: ogni giorno a un orario casuale tra le 02:00 e le 07:00 (ora locale), per non concentrare il carico del cloud.
//             Salta se l'ultima sincronizzazione ha meno di 12 ore.
//   Recupero: se l'ultima sincronizzazione ha più di 24 ore (PC spento di notte, ripresa dalla sospensione, nuovo accesso)
//             si sincronizza subito, senza che l'utente apra l'app.
// Se il cloud non è raggiungibile si riprova dopo 30 minuti (6 ore se il token non è più valido).
// L'intervallo minimo tra due sync (globals::SYNC_MIN_INTERVAL_SECS) vale sempre.

use crate::logdb;
use crate::secure_store;
use crate::sync::{self, SESSION_EXPIRED};
use chrono::{Duration as TimeSpan, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const STARTUP_DELAY_SECS: u64 = 20;
const CHECK_EVERY_SECS: u64 = 60;
const WINDOW_START_HOUR: u32 = 2;
const WINDOW_END_HOUR: u32 = 7;
const CATCH_UP_AFTER_SECS: i64 = 24 * 3600;
const NIGHT_MIN_AGE_SECS: i64 = 12 * 3600;
const RETRY_AFTER_SECS: u64 = 30 * 60;
const SESSION_RETRY_AFTER_SECS: u64 = 6 * 3600;
const NO_TOKEN_RETRY_AFTER_SECS: u64 = 10 * 60;

struct State {
	// Orario (locale) della prossima sincronizzazione notturna; solo in memoria, si ricalcola a ogni avvio
	plan: Option<NaiveDateTime>,
	retry_at: Option<Instant>,
}

pub fn start(app: AppHandle) {
	tauri::async_runtime::spawn(async move {
		tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SECS)).await;
		let mut state = State { plan: None, retry_at: None };
		loop {
			check(&app, &mut state).await;
			tokio::time::sleep(Duration::from_secs(CHECK_EVERY_SECS)).await;
		}
	});
}

async fn check(app: &AppHandle, state: &mut State) {
	if let Some(until) = state.retry_at {
		if Instant::now() < until { return; }
		state.retry_at = None;
	}
	let status = match sync::status() {
		Ok(status) => status,
		Err(_) => return,
	};
	if status.running || status.wait_secs > 0 { return; }

	let now = Local::now().naive_local();
	let last = match &status.last_sync_on {
		Some(text) => utc_text_to_local(text),
		None => None,
	};
	let age = last.map(|moment| (now - moment).num_seconds());

	let plan = match state.plan {
		Some(plan) => plan,
		None => {
			let plan = plan_next(now, last);
			state.plan = Some(plan);
			plan
		}
	};
	let night_reached = now >= plan;
	if night_reached { state.plan = None; }
	let catch_up = is_catch_up_due(age);
	let night = night_reached && is_night_due(age);
	if !catch_up && !night { return; }
	// Il keychain si legge solo qui, quando una sincronizzazione è davvero dovuta (e al massimo ogni 10 minuti se manca il token)
	if secure_store::get(secure_store::KEY_TOKEN).unwrap_or_default().is_empty() {
		state.retry_at = Some(Instant::now() + Duration::from_secs(NO_TOKEN_RETRY_AFTER_SECS));
		return;
	}
	let reason = if catch_up { "recupero" } else { "notturna" };
	run(app, state, reason).await;
}

async fn run(app: &AppHandle, state: &mut State, reason: &str) {
	let _ = logdb::write(&format!("Sincronizzazione automatica avviata ({})", reason));
	let emitter = app.clone();
	let result = sync::run(&move |progress| { let _ = emitter.emit("sync-progress", progress); }).await;
	match result {
		Ok(_) => {
			let _ = app.emit("sync-finished", true);
		}
		Err(error) => {
			let seconds = if error == SESSION_EXPIRED { SESSION_RETRY_AFTER_SECS } else { RETRY_AFTER_SECS };
			state.retry_at = Some(Instant::now() + Duration::from_secs(seconds));
			let _ = app.emit("sync-finished", false);
		}
	}
}

// Mai sincronizzato (None) o ultimo sync più vecchio di 24 ore
fn is_catch_up_due(age_secs: Option<i64>) -> bool {
	match age_secs {
		Some(seconds) => seconds >= CATCH_UP_AFTER_SECS,
		None => true,
	}
}

// La sincronizzazione notturna salta se l'ultima è recente
fn is_night_due(age_secs: Option<i64>) -> bool {
	match age_secs {
		Some(seconds) => seconds >= NIGHT_MIN_AGE_SECS,
		None => true,
	}
}

// Prossimo orario notturno casuale (ora locale). Oggi se la finestra 02:00-07:00 non è ancora finita
// e oggi non si è già sincronizzato; altrimenti domani.
fn plan_next(now: NaiveDateTime, last: Option<NaiveDateTime>) -> NaiveDateTime {
	let mut day = now.date();
	let synced_today = match last {
		Some(moment) => moment.date() >= day,
		None => false,
	};
	if now >= at_hour(day, WINDOW_END_HOUR) || synced_today { day = day.succ_opt().unwrap_or(day); }
	let window_start = at_hour(day, WINDOW_START_HOUR);
	let window_end = at_hour(day, WINDOW_END_HOUR);
	let soonest = now + TimeSpan::minutes(1);
	let earliest = if soonest > window_start { soonest } else { window_start };
	let span = (window_end - earliest).num_seconds().max(1) as u64;
	earliest + TimeSpan::seconds((random_u64() % span) as i64)
}

fn at_hour(day: NaiveDate, hour: u32) -> NaiveDateTime {
	day.and_hms_opt(hour, 0, 0).unwrap_or(NaiveDateTime::MIN)
}

// "YYYY-MM-DD HH:MM:SS" (UTC, come in SyncState) -> ora locale
fn utc_text_to_local(text: &str) -> Option<NaiveDateTime> {
	let utc = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").ok()?;
	Some(Utc.from_utc_datetime(&utc).with_timezone(&Local).naive_local())
}

fn random_u64() -> u64 {
	RandomState::new().build_hasher().finish()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn moment(text: &str) -> NaiveDateTime {
		NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").unwrap()
	}

	#[test]
	fn plan_stays_in_night_window() {
		let cases = [
			("2026-11-08 11:55", None, "2026-11-09"),
			("2026-11-08 00:30", None, "2026-11-08"),
			("2026-11-08 03:10", None, "2026-11-08"),
			("2026-11-08 03:10", Some("2026-11-08 02:40"), "2026-11-09"),
			("2026-11-08 08:00", Some("2026-11-07 03:00"), "2026-11-09"),
		];
		for (now, last, day) in cases {
			for _ in 0..200 {
				let plan = plan_next(moment(now), last.map(moment));
				assert_eq!(plan.date().to_string(), day);
				assert!(plan >= moment(now) && plan >= at_hour(plan.date(), 2) && plan < at_hour(plan.date(), 7), "{} -> {}", now, plan);
			}
		}
	}

	#[test]
	fn catch_up_and_night_thresholds() {
		assert!(is_catch_up_due(None));
		assert!(!is_catch_up_due(Some(23 * 3600)));
		assert!(is_catch_up_due(Some(24 * 3600)));
		assert!(!is_night_due(Some(3 * 3600)));
		assert!(is_night_due(Some(13 * 3600)));
	}
}