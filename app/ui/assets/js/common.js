// Condiviso da tutte le pagine. Le pagine interne hanno <body class="page" data-page="...">:
// all'apertura verificano lo sblocco (PIN), caricano i dati di sessione e disegnano la barra in alto.
// Quando è pronto, scatta l'evento "page-ready" con i dati di sessione in event.detail.

const invoke = window.__TAURI__.core.invoke;

// Layout con menu laterale (assets/css/shell.css): caricato qui così le pagine non cambiano.
(function loadShellStyle() {
	const link = document.createElement("link");
	link.rel = "stylesheet";
	link.href = "assets/css/shell.css";
	document.head.append(link);
	const icons = document.createElement("link");
	icons.rel = "stylesheet";
	icons.href = "assets/css/boxicons.css";
	document.head.append(icons);
})();

const SESSION_KEY = "optigest.session";
const LOGIN_PAGE = "index.html";

const FOOTER_LINKS = [
	{ page: "security", label: "Sicurezza" },
	{ page: "legal", label: "Legale" },
	{ page: "privacy", label: "Privacy" },
];

const ICON_LOCK = '<i class="bx bx-lock-alt"></i>';
const ICON_COPYRIGHT = '<i class="bx bx-copyright"></i>';

const ICON_HOME = '<i class="bx bx-bookmark"></i>';
const ICON_LIST = '<i class="bx bx-list-ol"></i>';
const ICON_CALENDAR = '<i class="bx bx-calendar"></i>';
const ICON_SETTINGS = '<i class="bx bx-cog"></i>';
const ICON_EXTERNAL = '<i class="bx bx-link-external"></i>';
const ICON_USER = '<i class="bx bx-user"></i>';
const ICON_LOGOUT = '<i class="bx bx-power-off"></i>';

// Menu laterale: sezioni come nel cloud. Le pagine di dettaglio evidenziano la voce dell'elenco.
const NAV_SECTIONS = [
	{
		title: "Pannello studio",
		items: [
			{ page: "homepage", label: "Home", href: "homepage.html", icon: ICON_HOME },
			{ page: "patients", label: "Pazienti", href: "patients.html", icon: ICON_LIST },
			{ page: "visits", label: "Visite", href: "visits.html", icon: ICON_LIST },
			{ page: "calendar", label: "Calendario", href: "calendar.html", icon: ICON_CALENDAR },
		],
	},
	{
		title: "Impostazioni",
		items: [
			{ page: "settings", label: "Impostazioni", href: "settings.html", icon: ICON_SETTINGS },
		],
	},
];
const PAGE_PARENT = { patient: "patients", visit: "visits" };

// Zoom: Cmd/Ctrl + (ingrandisce), Cmd/Ctrl - (riduce), Cmd/Ctrl 0 (100%), anche con Ctrl + rotella. Il livello è ricordato.
const ZOOM_KEY = "optigest.zoom";
const ZOOM_MIN = 0.6;
const ZOOM_MAX = 2.0;
const ZOOM_STEP = 0.1;

function readZoom() {
	try {
		const value = parseFloat(localStorage.getItem(ZOOM_KEY));
		if (value >= ZOOM_MIN && value <= ZOOM_MAX) { return value; }
	} catch (error) {
		return 1;
	}
	return 1;
}

async function applyZoom(factor) {
	const zoom = Math.round(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, factor)) * 100) / 100;
	try {
		localStorage.setItem(ZOOM_KEY, String(zoom));
	} catch (error) {
		// senza storage lo zoom vale solo per questa pagina
	}
	try {
		await invoke("set_zoom", { factor: zoom });
	} catch (error) {
		console.error("set_zoom", error);
	}
}

function onZoomKey(event) {
	if (!(event.metaKey || event.ctrlKey) || event.altKey) { return; }
	let next;
	if (event.key === "+" || event.key === "=") { next = readZoom() + ZOOM_STEP; }
	else if (event.key === "-" || event.key === "_") { next = readZoom() - ZOOM_STEP; }
	else if (event.key === "0") { next = 1; }
	else { return; }
	event.preventDefault();
	applyZoom(next);
}

function onZoomWheel(event) {
	if (!event.ctrlKey) { return; }
	event.preventDefault();
	applyZoom(readZoom() + (event.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP));
}

document.addEventListener("keydown", onZoomKey);
document.addEventListener("wheel", onZoomWheel, { passive: false });
if (readZoom() !== 1) { applyZoom(readZoom()); }

function $(id) {
	return document.getElementById(id);
}

function sleep(ms) {
	return new Promise(function (resolve) { setTimeout(resolve, ms); });
}

// 15480 -> 15'480 (separatore a scelta: nelle tabelle si usa il punto, 2.614)
function formatNumber(value, separator) {
	return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, separator || "'");
}

// "1990-05-01" -> "01/05/1990"
function formatDate(value) {
	if (!value) { return ""; }
	const parts = value.split("-");
	if (parts.length !== 3) { return value; }
	return parts[2] + "/" + parts[1] + "/" + parts[0];
}

// Il DB salva "YYYY-MM-DD HH:MM:SS" in UTC: si mostra nell'ora locale.
function formatDateTime(value) {
	if (!value) { return ""; }
	const date = new Date(value.replace(" ", "T") + "Z");
	if (isNaN(date.getTime())) { return value; }
	return date.toLocaleString("it-IT");
}

async function initPage() {
	const page = document.body.dataset.page;
	if (!page) { return; }
	if (!(await invoke("is_unlocked"))) { goToLogin(); return; }
	if ((await invoke("get_license_status")).blocked) { location.replace("license.html"); return; }
	const forceRefresh = document.body.dataset.refresh === "true";
	const info = await loadSession(forceRefresh);
	if (!info) { return; }
	const host = await invoke("get_hosts");
	renderShell(page, info, host.current);
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

// Menu laterale + barra in alto (stato e utente). Dipende da assets/css/shell.css.
function renderShell(page, info, host) {
	document.body.classList.add("shell");
	if (!$("sidebar")) { document.body.prepend(buildSidebar(PAGE_PARENT[page] || page, info.online)); }
	const bar = $("topbar");
	if (!bar) { return; }
	bar.classList.add("topbar");
	bar.replaceChildren();
	bar.append(element("span", "topbar-spacer", ""));
	bar.append(buildStatus(info.online, host));
	bar.append(buildUserMenu(info));
}

function buildSidebar(page, online) {
	const sidebar = document.createElement("aside");
	sidebar.id = "sidebar";
	sidebar.className = "sidebar";
	sidebar.append(buildBrand());
	sidebar.append(buildNav(page));
	sidebar.append(buildSidebarFooter(online));
	return sidebar;
}

// Logo bianco (assets/img/logo_w.svg, quello del cloud); se manca si usa quello scuro reso chiaro dal CSS.
function buildBrand() {
	const link = element("a", "sidebar-brand", "");
	link.href = "homepage.html";
	const image = document.createElement("img");
	image.src = "assets/img/logo_w.svg";
	image.alt = "OptiGest";
	image.addEventListener("error", function () {
		image.classList.add("logo-invert");
		image.src = "assets/img/logo.svg";
	}, { once: true });
	link.append(image);
	return link;
}

function buildNav(page) {
	const nav = element("nav", "sidebar-nav", "");
	for (const section of NAV_SECTIONS) {
		nav.append(element("div", "nav-title", section.title));
		for (const item of section.items) { nav.append(buildNavItem(item, item.page === page)); }
	}
	return nav;
}

function buildNavItem(item, active) {
	const link = element("a", "nav-item" + (active ? " active" : ""), "");
	link.href = item.href;
	link.innerHTML = item.icon;
	link.append(element("span", "", item.label));
	return link;
}

// In fondo al menu: link legali (browser di sistema), stato connessione, copyright.
function buildSidebarFooter(online) {
	const footer = element("div", "sidebar-footer", "");
	footer.append(buildFooterLinks());
	footer.append(buildFooterNote(ICON_LOCK, online ? "Connessione sicura" : "Modalità offline: dati locali"));
	footer.append(buildFooterNote(ICON_COPYRIGHT, "EosMedTech Srl"));
	return footer;
}

function buildFooterLinks() {
	const links = element("div", "footer-links", "");
	for (const item of FOOTER_LINKS) {
		const link = element("a", "", item.label);
		link.href = "#";
		link.addEventListener("click", function (event) {
			event.preventDefault();
			invoke("open_legal_page", { page: item.page }).catch(function () { /* nessun browser disponibile: niente da fare */ });
		});
		links.append(link);
	}
	return links;
}

// Le icone sono SVG fissi del codice (niente font esterni: l'app funziona anche offline).
function buildFooterNote(icon, text) {
	const note = element("span", "footer-note", "");
	note.innerHTML = icon;
	note.append(text);
	return note;
}

function buildStatus(online, host) {
	const status = element("div", "status " + (online ? "online" : "offline"), "");
	status.append(document.createElement("i"));
	status.append(element("span", "", (online ? "Online" : "Offline") + " · " + host));
	return status;
}

// Nome utente + avatar: al click si apre il menu con il Cloud e Disconnetti.
function buildUserMenu(info) {
	const wrapper = element("div", "user-menu", "");
	const button = element("button", "user-button", "");
	button.type = "button";
	button.append(element("span", "user-name", ((info.title ? info.title + " " : "") + (info.name || "")).trim() || "Utente"));
	const avatar = element("span", "user-avatar", "");
	avatar.innerHTML = ICON_USER;
	button.append(avatar);
	const dropdown = element("div", "user-dropdown", "");
	dropdown.hidden = true;
	dropdown.append(buildMenuLink("Apri il Cloud", ICON_EXTERNAL, function () { invoke("open_cloud").catch(function (error) { alert(String(error)); }); }));
	dropdown.append(buildMenuLink("Disconnetti", ICON_LOGOUT, logout));
	button.addEventListener("click", function (event) {
		event.stopPropagation();
		dropdown.hidden = !dropdown.hidden;
	});
	document.addEventListener("click", function () { dropdown.hidden = true; });
	wrapper.append(button, dropdown);
	return wrapper;
}

function buildMenuLink(label, icon, action) {
	const item = element("button", "dropdown-item", "");
	item.type = "button";
	item.innerHTML = icon;
	item.append(element("span", "", label));
	item.addEventListener("click", action);
	return item;
}

function element(tag, className, text) {
	const node = document.createElement(tag);
	if (className) { node.className = className; }
	node.textContent = text;
	return node;
}

function addWordButton(titleId, buildRequest) {
	const title = $(titleId);
	const button = element("button", "btn-primary word-button", "");
	button.type = "button";
	button.title = "Scarica riepilogo Word";
	button.innerHTML = '<i class="bx bx-printer"></i> Stampa';
	button.addEventListener("click", async function () {
		button.disabled = true;
		try {
			const path = await invoke("export_word", { request: buildRequest() });
			alert("Documento salvato in:\n" + path);
		} catch (error) {
			alert("Operazione non riuscita: " + String(error));
		} finally {
			button.disabled = false;
		}
	});
	title.classList.add("title-with-action");
	title.append(button);
}

// Dopo DOMContentLoaded tutti gli script della pagina (es. homepage.js) hanno già registrato i loro listener.
document.addEventListener("DOMContentLoaded", initPage);
