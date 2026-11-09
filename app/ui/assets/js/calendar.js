// Calendario: appuntamenti dal DB locale (sola visualizzazione), anche offline.
// Dipende da common.js (invoke, $, element) e da FullCalendar (assets/vendor, in locale).

const CALENDAR_START_HOUR = "07:00:00";
const CALENDAR_END_HOUR = "21:00:00";
const CALENDAR_SCROLL_TO = "08:00:00";
const CALENDAR_SLOT = "00:15:00";
// Chiavi di EventData che non interessa mostrare
const EVENT_HIDDEN_KEYS = ["gcalid", "color", "colour"];

let calendar = null;

document.addEventListener("page-ready", startCalendar);

function startCalendar() {
	calendar = new FullCalendar.Calendar($("calendar"), {
		locale: "it",
		firstDay: 1,
		initialView: "timeGridWeek",
		initialDate: new URLSearchParams(location.search).get("date") || undefined,
		headerToolbar: { left: "title", right: "prev,next today dayGridMonth,timeGridWeek,timeGridDay,listWeek" },
		height: "calc(100vh - 190px)",
		nowIndicator: true,
		slotMinTime: CALENDAR_START_HOUR,
		slotMaxTime: CALENDAR_END_HOUR,
		scrollTime: CALENDAR_SCROLL_TO,
		slotDuration: CALENDAR_SLOT,
		slotLabelFormat: { hour: "2-digit", minute: "2-digit", hour12: false },
		eventTimeFormat: { hour: "2-digit", minute: "2-digit", hour12: false },
		dayMaxEvents: true,
		editable: false,
		selectable: false,
		events: loadEvents,
		eventClick: onEventClick,
		eventDidMount: onEventMount,
	});
	calendar.render();
	// Dopo il caricamento dei font le larghezze cambiano: si ricalcola il layout.
	document.fonts.ready.then(function () { calendar.updateSize(); });
	$("event-close").addEventListener("click", closeEvent);
	$("event-overlay").addEventListener("click", function (event) { if (event.target === $("event-overlay")) { closeEvent(); } });
	document.addEventListener("keydown", function (event) { if (event.key === "Escape") { closeEvent(); } });
}

// FullCalendar chiede gli appuntamenti della finestra visibile; le date si passano come testo "locale", come nel DB.
function loadEvents(info, success, failure) {
	$("calendar-error").textContent = "";
	invoke("list_events", { start: toDbDate(info.start), end: toDbDate(info.end) })
		.then(function (rows) { success(rows.map(toCalendarEvent)); })
		.catch(function (error) {
			$("calendar-error").textContent = "Appuntamenti non disponibili: " + error;
			failure(error);
		});
}

function toCalendarEvent(row) {
	return {
		id: String(row.id),
		title: row.title || "(senza titolo)",
		start: row.start.replace(" ", "T"),
		end: row.end.replace(" ", "T"),
		extendedProps: { data: parseData(row.data) },
	};
}

function parseData(text) {
	try {
		const value = JSON.parse(text);
		if (value && typeof value === "object") { return value; }
	} catch (error) {
		return {};
	}
	return {};
}

// Date -> "YYYY-MM-DD HH:MM:SS" (ora locale, senza fuso)
function toDbDate(date) {
	const two = function (value) { return String(value).padStart(2, "0"); };
	return date.getFullYear() + "-" + two(date.getMonth() + 1) + "-" + two(date.getDate()) + " " + two(date.getHours()) + ":" + two(date.getMinutes()) + ":" + two(date.getSeconds());
}

function onEventMount(info) {
	info.el.title = info.event.title;
}

function onEventClick(info) {
	info.jsEvent.preventDefault();
	showEvent(info.event);
}

function showEvent(event) {
	$("event-title").textContent = event.title;
	$("event-when").textContent = describeWhen(event.start, event.end);
	const list = $("event-fields");
	list.replaceChildren();
	const data = event.extendedProps.data || {};
	for (const key of Object.keys(data)) {
		const text = describeValue(data[key]);
		if (EVENT_HIDDEN_KEYS.includes(key.toLowerCase()) || text === "") { continue; }
		list.append(element("dt", "", humanize(key)));
		list.append(element("dd", "", text));
	}
	$("event-overlay").hidden = false;
}

function closeEvent() {
	$("event-overlay").hidden = true;
}

// "sabato 7 novembre 2026, 08:15 - 08:30"
function describeWhen(start, end) {
	if (!start) { return ""; }
	const day = start.toLocaleDateString("it-IT", { weekday: "long", day: "numeric", month: "long", year: "numeric" });
	const time = { hour: "2-digit", minute: "2-digit" };
	let text = day + ", " + start.toLocaleTimeString("it-IT", time);
	if (end) { text += " - " + end.toLocaleTimeString("it-IT", time); }
	return text;
}

function describeValue(value) {
	if (value === null || value === undefined || value === false) { return ""; }
	if (value === true) { return "Sì"; }
	if (typeof value === "object") { return Object.keys(value).length === 0 ? "" : JSON.stringify(value); }
	return String(value).trim();
}

// "patient_name" -> "Patient name"
function humanize(key) {
	const text = key.replace(/[_-]+/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2").trim();
	return text.charAt(0).toUpperCase() + text.slice(1);
}