// Homepage: saluto, widget con i conteggi (DB locale), sincronizzazione con il cloud e risposta del login (debug).

const listen = window.__TAURI__.event.listen;
const AUTO_SYNC_KEY = "optigest.autoSyncDone";

const TABLE_LABELS = { patient: "Pazienti", visit: "Visite" };

let syncing = false;

document.addEventListener("page-ready", onReady);

function onReady(event) {
	const info = event.detail;
	$("home-greeting").textContent = greeting() + " " + (info.title || "Dott.") + " " + info.name;
	$("home-raw").textContent = info.raw || "(nessuna risposta: offline, mostrati i dati in cache)";
	$("btn-sync").addEventListener("click", startSync);
	listen("sync-progress", onSyncProgress);
	loadCounts();
	showLastSync();
	if (info.online && !autoSyncDone()) { markAutoSyncDone(); startSync(); }
}

// Conteggi dei record non eliminati: funzionano anche offline.
async function loadCounts() {
	try {
		const counts = await invoke("get_counts");
		$("count-patients").textContent = formatNumber(counts.patients);
		$("count-visits").textContent = formatNumber(counts.visits);
	} catch (error) {
		$("count-patients").textContent = "?";
		$("count-visits").textContent = "?";
	}
}

async function showLastSync() {
	let status;
	try {
		status = await invoke("get_sync_status");
	} catch (error) {
		return;
	}
	if (syncing) { return; }
	if (status.running) { $("sync-status").textContent = "Sincronizzazione in corso..."; return; }
	if (!status.last_sync_on) { $("sync-status").textContent = "Mai sincronizzato: i dati locali sono vuoti o di prova."; return; }
	const suffix = status.full_pending ? " (scarico completo da terminare)" : "";
	$("sync-status").textContent = "Ultimo aggiornamento: " + formatDateTime(status.last_sync_on) + suffix;
}

async function startSync() {
	const button = $("btn-sync");
	syncing = true;
	button.disabled = true;
	$("sync-status").textContent = "Sincronizzazione in corso...";
	$("sync-progress").textContent = "";
	try {
		const report = await invoke("sync_now");
		syncing = false;
		$("sync-progress").textContent = summary(report);
		showLastSync();
	} catch (error) {
		syncing = false;
		showSyncError(String(error));
	}
	button.disabled = false;
	loadCounts();
}

// Avanzamento: righe scaricate finora per tabella; i conteggi si aggiornano mentre arrivano.
function onSyncProgress(event) {
	const progress = event.payload;
	$("sync-progress").textContent = TABLE_LABELS[progress.table] + ": " + formatNumber(progress.rows, ".") + " scaricati";
	loadCounts();
}

function summary(report) {
	const kind = report.full ? "Scarico completo" : "Aggiornamento";
	let text = kind + ": " + formatNumber(report.patients, ".") + " pazienti, " + formatNumber(report.visits, ".") + " visite";
	if (report.removed > 0) { text += "; " + formatNumber(report.removed, ".") + " record locali non presenti sul cloud rimossi"; }
	return text;
}

function showSyncError(message) {
	if (message === "SESSION_EXPIRED") { goToLogin(); return; }
	$("sync-status").textContent = "Sincronizzazione non riuscita: " + message + ". Restano i dati locali.";
	$("sync-progress").textContent = "";
}

function autoSyncDone() {
	try {
		return sessionStorage.getItem(AUTO_SYNC_KEY) === "1";
	} catch (error) {
		return false;
	}
}

function markAutoSyncDone() {
	try {
		sessionStorage.setItem(AUTO_SYNC_KEY, "1");
	} catch (error) {
		// senza storage la sincronizzazione automatica riparte a ogni apertura della home
	}
}

function greeting() {
	const hour = new Date().getHours();
	if (hour < 13) { return "Buon giorno"; }
	if (hour < 18) { return "Buon pomeriggio"; }
	return "Buona sera";
}