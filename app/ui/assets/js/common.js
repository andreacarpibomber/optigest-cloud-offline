// Condiviso da tutte le pagine. Le pagine interne hanno <body class="page" data-page="...">:
// all'apertura verificano lo sblocco (PIN), caricano i dati di sessione e disegnano la barra in alto.
// Quando è pronto, scatta l'evento "page-ready" con i dati di sessione in event.detail.

const invoke = window.__TAURI__.core.invoke;

const SESSION_KEY = "optigest.session";
const LOGIN_PAGE = "index.html";

const NAV = [
	{ page: "homepage", label: "Home", href: "homepage.html" },
	{ page: "patients", label: "Pazienti", href: "patients.html" },
	{ page: "visits", label: "Visite", href: "visits.html" },
];

function $(id) {
	return document.getElementById(id);
}

function sleep(ms) {
	return new Promise(function (resolve) { setTimeout(resolve, ms); });
}

// 15480 -> 15'480
function formatNumber(value) {
	return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, "'");
}

async function initPage() {
	const page = document.body.dataset.page;
	if (!page) { return; }
	if (!(await invoke("is_unlocked"))) { goToLogin(); return; }
	const forceRefresh = document.body.dataset.refresh === "true";
	const info = await loadSession(forceRefresh);
	if (!info) { return; }
	const host = await invoke("get_hosts");
	renderTopbar(page, info, host.current);
	document.dispatchEvent(new CustomEvent("page-ready", { detail: info }));
}

// Dati di sessione (studio, utente, online): sessionStorage tra le pagine, cloud solo se mancano o se richiesto.
async function loadSession(forceRefresh) {
	if (!forceRefresh) {
		const stored = readStored();
		if (stored) { return stored; }
	}
	let info;
	try {
		info = await invoke("connect_upstream");
	} catch (error) {
		goToLogin();
		return null;
	}
	if (!info.valid) { goToLogin(); return null; }
	writeStored(info);
	return info;
}

function readStored() {
	try {
		const text = sessionStorage.getItem(SESSION_KEY);
		if (text) { return JSON.parse(text); }
	} catch (error) {
		return null;
	}
	return null;
}

function writeStored(info) {
	try {
		sessionStorage.setItem(SESSION_KEY, JSON.stringify(info));
	} catch (error) {
		// senza storage si rifà la chiamata a ogni pagina
	}
}

function clearStored() {
	try {
		sessionStorage.removeItem(SESSION_KEY);
	} catch (error) {
		// niente da cancellare
	}
}

function goToLogin() {
	clearStored();
	location.replace(LOGIN_PAGE);
}

async function logout() {
	await invoke("logout");
	goToLogin();
}

function renderTopbar(page, info, host) {
	const bar = $("topbar");
	if (!bar) { return; }
	bar.classList.add("topbar");
	bar.replaceChildren();
	bar.append(element("span", "topbar-org", info.org || "Studio"));
	bar.append(element("span", "topbar-user", info.name || "Utente"));
	bar.append(buildNav(page));
	bar.append(element("span", "topbar-spacer", ""));
	bar.append(buildStatus(info.online, host));
	bar.append(buildLogoutButton());
}

function buildNav(page) {
	const nav = element("nav", "topbar-nav", "");
	for (const item of NAV) {
		const link = element("a", item.page === page ? "active" : "", item.label);
		link.href = item.href;
		nav.append(link);
	}
	return nav;
}

function buildStatus(online, host) {
	const status = element("div", "status " + (online ? "online" : "offline"), "");
	status.append(document.createElement("i"));
	status.append(element("span", "", (online ? "Online" : "Offline") + " · " + host));
	return status;
}

function buildLogoutButton() {
	const button = element("button", "link", "Disconnetti");
	button.type = "button";
	button.addEventListener("click", logout);
	return button;
}

function element(tag, className, text) {
	const node = document.createElement(tag);
	if (className) { node.className = className; }
	node.textContent = text;
	return node;
}

// Dopo DOMContentLoaded tutti gli script della pagina (es. homepage.js) hanno già registrato i loro listener.
document.addEventListener("DOMContentLoaded", initPage);