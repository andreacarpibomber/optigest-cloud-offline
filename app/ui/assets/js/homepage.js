// Homepage: saluto, widget con i conteggi (DB locale), sincronizzazione con il cloud.

const listen = window.__TAURI__.event.listen;
const AUTO_SYNC_KEY = "optigest.autoSyncDone";

const TABLE_LABELS = { patient: "Pazienti", visit: "Visite", event: "Appuntamenti" };

let syncing = false;

document.addEventListener("page-ready", onReady);

function onReady(event) {
	const info = event.detail;
	$("home-greeting").textContent = greeting() + " " + (info.title || "Dott.") + " " + info.name;
	$("btn-sync").addEventListener("click", startSync);
	listen("sync-progress", onSyncProgress);
	loadCounts();
	showLastSync();
	showBackupAlert();
	showLicenseAlert();
	showVersion();
	if (info.online && !autoSyncDone()) { markAutoSyncDone(); autoSync(); }
}

// Sync automatico all'apertura: solo se l'ultimo è abbastanza vecchio (il limite è applicato anche dal backend).
async function autoSync() {
	try {
		const status = await invoke("get_sync_status");
		if (status.wait_secs > 0 || status.running) { return; }
	} catch (error) {
		return;
	}
	startSync();
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
		showBackupAlert();
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
	let text = kind + ": " + formatNumber(report.patients, ".") + " pazienti, " + formatNumber(report.visits, ".") + " visite, " + formatNumber(report.events, ".") + " appuntamenti";
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

// Avviso se l'ultimo backup del DB locale non è riuscito (resta finché non ne riesce uno).
async function showBackupAlert() {
	const alertBox = $("backup-alert");
	let status;
	try {
		status = await invoke("get_backup_status");
	} catch (error) {
		return;
	}
	alertBox.hidden = !status.last_error;
	if (!status.last_error) { return; }
	alertBox.textContent = "Backup del database locale non riuscito: " + status.last_error + " ";
	const link = element("a", "", "Impostazioni");
	link.href = "settings.html";
	alertBox.append(link);
}

// Avviso negli ultimi giorni di licenza.
async function showLicenseAlert() {
	let status;
	try {
		status = await invoke("get_license_status");
	} catch (error) {
		return;
	}
	if (status.state !== "expiring") { return; }
	const box = $("license-alert");
	const days = status.days_left;
	box.textContent = "Licenza in scadenza il " + formatDate(status.valid_until) + (days <= 0 ? " (oggi)" : " (tra " + days + (days === 1 ? " giorno" : " giorni") + ")") + ": contattare EosMedTech per il rinnovo.";
	box.hidden = false;
}

// Versione dell'app (Cargo.toml).
async function showVersion() {
	try {
		$("app-version").textContent = "Versione " + await invoke("app_version");
	} catch (error) {
		console.error("app_version", error);
	}
}

function greeting() {
	const hour = new Date().getHours();
	if (hour < 13) { return "Buon giorno"; }
	if (hour < 18) { return "Buon pomeriggio"; }
	return "Buona sera";
}