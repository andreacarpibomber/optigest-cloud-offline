// Elenco pazienti: filtri per colonna, ordinamento, paginazione. Clic sulla riga -> patient.html?id=...
// Filtri e ordinamento si ricordano finché la sessione è aperta (si torna dalla scheda paziente allo stesso elenco).

const FILTER_INPUTS = {
	id: "f-id",
	surname: "f-surname",
	name: "f-name",
	birthday: "f-birthday",
	fiscal_code: "f-fiscal",
};
const PAGE_SIZE = 10;
const SEARCH_DELAY_MS = 250;
const STATE_KEY = "optigest.patients";
const ICON_OPEN = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><path d="M15 3h6v6"/><path d="M10 14 21 3"/></svg>';

const state = { sort: "id", desc: true, page: 1 };
let searchTimer = null;
let requestSeq = 0;

document.addEventListener("page-ready", onReady);

function onReady() {
	restoreState();
	for (const field in FILTER_INPUTS) { bindFilter(field); }
	document.querySelectorAll("th[data-sort]").forEach(bindSort);
	showSortState();
	load();
}

function bindFilter(field) {
	const input = $(FILTER_INPUTS[field]);
	const eventName = input.type === "date" ? "change" : "input";
	input.addEventListener(eventName, onFilterChange);
}

function bindSort(header) {
	header.querySelector("button").addEventListener("click", function () { changeSort(header.dataset.sort); });
}

function onFilterChange() {
	clearTimeout(searchTimer);
	searchTimer = setTimeout(applyFilters, SEARCH_DELAY_MS);
}

function applyFilters() {
	state.page = 1;
	saveState();
	load();
}

function changeSort(field) {
	if (state.sort === field) { state.desc = !state.desc; } else { state.sort = field; state.desc = false; }
	state.page = 1;
	saveState();
	showSortState();
	load();
}

function goToPage(page) {
	state.page = page;
	saveState();
	load();
}

function buildQuery() {
	const query = { sort: state.sort, desc: state.desc, page: state.page, page_size: PAGE_SIZE };
	for (const field in FILTER_INPUTS) { query[field] = $(FILTER_INPUTS[field]).value.trim(); }
	return query;
}

async function load() {
	const seq = ++requestSeq;
	let result;
	try {
		result = await invoke("search_patients", { query: buildQuery() });
	} catch (error) {
		showError(String(error));
		return;
	}
	if (seq !== requestSeq) { return; }
	// Filtri cambiati mentre si era a una pagina alta: si torna all'ultima pagina esistente.
	if (result.rows.length === 0 && result.total > 0 && state.page > 1) { goToPage(Math.ceil(result.total / PAGE_SIZE)); return; }
	showError("");
	renderRows(result.rows);
	renderInfo(result);
	renderPager(result);
}

function renderRows(rows) {
	const body = $("patients-body");
	body.replaceChildren();
	for (const patient of rows) { body.append(buildRow(patient)); }
}

function buildRow(patient) {
	const row = document.createElement("tr");
	row.className = "data-row";
	row.append(cell(String(patient.id)));
	row.append(cell(patient.surname));
	row.append(cell(patient.name));
	row.append(cell(formatDate(patient.birthday)));
	row.append(cell(patient.fiscal_code));
	row.append(actionCell(patient.id));
	row.addEventListener("click", function () { openPatient(patient.id); });
	return row;
}

function cell(text) {
	const td = document.createElement("td");
	td.textContent = text;
	return td;
}

function actionCell(id) {
	const td = document.createElement("td");
	td.className = "col-action";
	const link = document.createElement("a");
	link.href = patientUrl(id);
	link.className = "row-open";
	link.title = "Apri paziente";
	link.innerHTML = ICON_OPEN;
	link.addEventListener("click", function (event) { event.stopPropagation(); });
	td.append(link);
	return td;
}

function patientUrl(id) {
	return "patient.html?id=" + encodeURIComponent(id);
}

function openPatient(id) {
	location.href = patientUrl(id);
}

function renderInfo(result) {
	if (result.total === 0) { $("results-info").textContent = "Nessun risultato"; return; }
	const from = (result.page - 1) * result.page_size + 1;
	const to = from + result.rows.length - 1;
	$("results-info").textContent = "Risultati da " + from + " a " + to + " di " + formatNumber(result.total, ".");
}

function renderPager(result) {
	const pager = $("pager");
	pager.replaceChildren();
	const last = Math.ceil(result.total / result.page_size);
	if (last <= 1) { return; }
	pager.append(pageButton("‹", result.page - 1, result.page > 1, false));
	for (const page of pageWindow(result.page, last)) {
		if (page === 0) { pager.append(gap()); continue; }
		pager.append(pageButton(String(page), page, true, page === result.page));
	}
	pager.append(pageButton("›", result.page + 1, result.page < last, false));
}

// Numeri di pagina da mostrare; 0 = "…"
function pageWindow(current, last) {
	if (last <= 7) { return range(1, last); }
	if (current <= 4) { return range(1, 5).concat([0, last]); }
	if (current >= last - 3) { return [1, 0].concat(range(last - 4, last)); }
	return [1, 0, current - 1, current, current + 1, 0, last];
}

function range(from, to) {
	const numbers = [];
	for (let n = from; n <= to; n++) { numbers.push(n); }
	return numbers;
}

function pageButton(label, page, enabled, current) {
	const button = document.createElement("button");
	button.type = "button";
	button.textContent = label;
	button.disabled = !enabled;
	if (current) { button.className = "current"; }
	if (enabled && !current) { button.addEventListener("click", function () { goToPage(page); }); }
	return button;
}

function gap() {
	const span = document.createElement("span");
	span.textContent = "…";
	return span;
}

function showSortState() {
	document.querySelectorAll("th[data-sort]").forEach(markSort);
}

function markSort(header) {
	header.classList.remove("sorted-asc", "sorted-desc");
	if (header.dataset.sort !== state.sort) { return; }
	header.classList.add(state.desc ? "sorted-desc" : "sorted-asc");
}

function showError(message) {
	$("patients-error").textContent = message;
}

function saveState() {
	const saved = { sort: state.sort, desc: state.desc, page: state.page };
	for (const field in FILTER_INPUTS) { saved[field] = $(FILTER_INPUTS[field]).value; }
	try {
		sessionStorage.setItem(STATE_KEY, JSON.stringify(saved));
	} catch (error) {
		// senza storage i filtri non si ricordano
	}
}

function restoreState() {
	let saved;
	try {
		saved = JSON.parse(sessionStorage.getItem(STATE_KEY));
	} catch (error) {
		return;
	}
	if (!saved) { return; }
	state.sort = saved.sort || state.sort;
	state.desc = saved.desc !== false;
	state.page = saved.page || 1;
	for (const field in FILTER_INPUTS) { $(FILTER_INPUTS[field]).value = saved[field] || ""; }
}