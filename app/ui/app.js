// Flusso (schema su carta):
// splash -> sessione salvata? NO -> pair
//                             SI -> PIN presente? SI -> sblocco / NO -> imposta PIN
//        -> home (studio, utente, saluto, conteggi pazienti/visite)

const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const MIN_SPLASH_MS = 1800;
const HOST_LONG_PRESS_MS = 3000;

let pinMode = "unlock";
let hostTimer = null;
let currentHost = "";

function $(id) {
	return document.getElementById(id);
}

function show(viewId) {
	document.querySelectorAll(".view").forEach(hideView);
	$(viewId).classList.add("active");
}

function hideView(view) {
	view.classList.remove("active");
}

function sleep(ms) {
	return new Promise(function (resolve) { setTimeout(resolve, ms); });
}

async function start() {
	show("view-splash");
	$("splash-note").textContent = "";
	$("btn-repair").classList.add("hidden");
	const began = Date.now();
	await loadHost();
	const state = await invoke("boot_state");
	const remaining = Math.max(0, MIN_SPLASH_MS - (Date.now() - began));
	await sleep(remaining);
	route(state);
}

function route(state) {
	if (state === "pair") { startPair(); return; }
	if (state === "set_pin") { showPin("set"); return; }
	showPin("unlock");
}

async function startPair() {
	show("view-splash");
	$("splash-note").textContent = "Completa l'accesso a OptiGest Cloud nella finestra aperta.";
	$("btn-repair").classList.remove("hidden");
	try {
		await invoke("start_pair");
	} catch (error) {
		$("splash-note").textContent = "Impossibile aprire l'accesso: " + error;
	}
}

function showPin(mode) {
	pinMode = mode;
	show("view-pin");
	$("pin-error").textContent = "";
	$("pin-input").value = "";
	$("pin-confirm").value = "";
	if (mode === "set") {
		$("pin-title").textContent = "Imposta il PIN di sblocco";
		$("pin-confirm").classList.remove("hidden");
	} else {
		$("pin-title").textContent = "Inserisci il PIN";
		$("pin-confirm").classList.add("hidden");
	}
	$("pin-input").focus();
}

async function submitPin(event) {
	event.preventDefault();
	const pin = $("pin-input").value;
	if (pinMode === "set") { await submitNewPin(pin); return; }
	await submitUnlock(pin);
}

async function submitNewPin(pin) {
	if (pin !== $("pin-confirm").value) {
		$("pin-error").textContent = "I due PIN non coincidono.";
		return;
	}
	try {
		await invoke("set_pin", { pin: pin });
	} catch (error) {
		$("pin-error").textContent = String(error);
		return;
	}
	await openHome();
}

async function submitUnlock(pin) {
	let outcome;
	try {
		outcome = await invoke("verify_pin", { pin: pin });
	} catch (error) {
		$("pin-error").textContent = String(error);
		return;
	}
	if (outcome.status === "ok") { await openHome(); return; }
	if (outcome.status === "locked") { await start(); return; }
	$("pin-input").value = "";
	$("pin-error").textContent = "PIN errato. Tentativi rimasti: " + outcome.remaining;
}

async function openHome() {
	await loadHost();
	show("view-home");
	setStatus("", "Connessione...");
	$("home-org").textContent = "";
	$("home-user").textContent = "";
	$("home-greeting").textContent = "";
	$("home-raw").textContent = "";
	loadCounts();
	const info = await invoke("connect_upstream");
	if (!info.valid) { await start(); return; }
	$("home-org").textContent = info.org || "Studio";
	$("home-user").textContent = info.name || "Utente";
	$("home-greeting").textContent = greeting() + " " + (info.title || "Dott.") + " " + info.name;
	$("home-raw").textContent = info.raw || "(nessuna risposta: offline, mostrati i dati in cache)";
	if (info.online) { setStatus("online", "Online"); return; }
	setStatus("offline", "Offline");
}

// Conteggi dal DB locale (record non eliminati): funzionano anche offline.
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

function greeting() {
	const hour = new Date().getHours();
	if (hour < 13) { return "Buon giorno"; }
	if (hour < 18) { return "Buon pomeriggio"; }
	return "Buona sera";
}

// 15480 -> 15'480
function formatNumber(value) {
	return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, "'");
}

function setStatus(kind, label) {
	const status = $("home-status");
	status.className = "status " + kind;
	status.querySelector("span").textContent = currentHost ? label + " · " + currentHost : label;
}

// Host corrente (da config.json): mostrato sullo splash e accanto allo stato in home.
async function loadHost() {
	const info = await invoke("get_hosts");
	currentHost = info.current;
	$("splash-host").textContent = currentHost;
}

async function logout() {
	await invoke("logout");
	await start();
}

// Selettore host: long press 3s sul logo dello splash (come la fascia in alto nell'app Swift).
async function openHostPicker() {
	const info = await invoke("get_hosts");
	const list = $("host-list");
	list.innerHTML = "";
	info.hosts.forEach(function (host) {
		const item = document.createElement("li");
		const button = document.createElement("button");
		button.type = "button";
		button.textContent = host === info.current ? host + " ✓" : host;
		button.addEventListener("click", function () { chooseHost(host); });
		item.appendChild(button);
		list.appendChild(item);
	});
	$("host-overlay").classList.remove("hidden");
}

async function chooseHost(host) {
	await invoke("set_host", { host: host });
	await loadHost();
	$("host-overlay").classList.add("hidden");
}

function logoPressStart() {
	hostTimer = setTimeout(openHostPicker, HOST_LONG_PRESS_MS);
}

function logoPressEnd() {
	clearTimeout(hostTimer);
}

$("pin-form").addEventListener("submit", submitPin);
$("btn-repair").addEventListener("click", startPair);
$("btn-logout").addEventListener("click", logout);
$("host-cancel").addEventListener("click", function () { $("host-overlay").classList.add("hidden"); });
$("logo").addEventListener("pointerdown", logoPressStart);
$("logo").addEventListener("pointerup", logoPressEnd);
$("logo").addEventListener("pointerleave", logoPressEnd);

listen("pair-completed", start);

start();