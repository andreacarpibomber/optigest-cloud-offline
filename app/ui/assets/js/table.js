// Tabella con filtri per colonna, ordinamento e paginazione, alimentata da un comando dell'API (ricerca lato DB).
// Usata da patients.js e visits.js. Dipende da common.js (invoke, $, formatNumber).
//
// config:
//   command      nome del comando (es. "search_patients"), riceve { query }
//   filters      { campo: "id-dell-input" }; il valore di ogni input finisce in query[campo]
//   defaultSort  campo di ordinamento iniziale (decrescente)
//   cells(row)   elenco dei testi delle celle della riga
//   url(row)     indirizzo aperto cliccando la riga o l'icona
//   stateKey     chiave di sessionStorage per ricordare filtri/ordinamento/pagina
//   extraQuery() (facoltativo) campi fissi aggiunti alla query (es. { patient_id })
//   rowClass(row) (facoltativo) classe CSS aggiunta alla riga (es. per evidenziarne una)
//   autoStart    false = la tabella parte solo chiamando start() (quando serve un dato caricato dopo, es. l'ID paziente)
//   onRender()   (facoltativo) chiamata dopo ogni caricamento di righe
//   actions      false = senza colonna "Apri" (si apre cliccando la riga)
// La pagina deve avere: #table-body, #table-error, #results-info, #pager, th[data-sort], e gli input dei filtri.

const TABLE_PAGE_SIZE = 10;
const TABLE_SEARCH_DELAY_MS = 250;
const ICON_OPEN = '<i class="bx bx-link-external"></i>';

class DataTable {
	constructor(config) {
		this.config = config;
		this.state = { sort: config.defaultSort, desc: true, page: 1 };
		this.searchTimer = null;
		this.requestSeq = 0;
		if (config.autoStart !== false) { document.addEventListener("page-ready", () => this.start()); }
	}

	start() {
		this.restoreState();
		for (const field in this.config.filters) { this.bindFilter(field); }
		document.querySelectorAll("th[data-sort]").forEach((header) => this.bindSort(header));
		this.showSortState();
		this.load();
	}

	bindFilter(field) {
		const input = $(this.config.filters[field]);
		const eventName = input.type === "date" ? "change" : "input";
		input.addEventListener(eventName, () => this.onFilterChange());
	}

	bindSort(header) {
		header.querySelector("button").addEventListener("click", () => this.changeSort(header.dataset.sort));
	}

	onFilterChange() {
		clearTimeout(this.searchTimer);
		this.searchTimer = setTimeout(() => this.applyFilters(), TABLE_SEARCH_DELAY_MS);
	}

	applyFilters() {
		this.state.page = 1;
		this.saveState();
		this.load();
	}

	changeSort(field) {
		if (this.state.sort === field) {
			this.state.desc = !this.state.desc;
		} else {
			this.state.sort = field;
			this.state.desc = false;
		}
		this.state.page = 1;
		this.saveState();
		this.showSortState();
		this.load();
	}

	goToPage(page) {
		this.state.page = page;
		this.saveState();
		this.load();
	}

	buildQuery() {
		const query = { sort: this.state.sort, desc: this.state.desc, page: this.state.page, page_size: TABLE_PAGE_SIZE };
		for (const field in this.config.filters) { query[field] = $(this.config.filters[field]).value.trim(); }
		if (this.config.extraQuery) { Object.assign(query, this.config.extraQuery()); }
		return query;
	}

	async load() {
		const seq = ++this.requestSeq;
		let result;
		try {
			result = await invoke(this.config.command, { query: this.buildQuery() });
		} catch (error) {
			this.showError(String(error));
			return;
		}
		if (seq !== this.requestSeq) { return; }
		// Filtri cambiati mentre si era a una pagina alta: si torna all'ultima pagina esistente.
		if (result.rows.length === 0 && result.total > 0 && this.state.page > 1) { this.goToPage(Math.ceil(result.total / TABLE_PAGE_SIZE)); return; }
		this.showError("");
		this.renderRows(result.rows);
		this.renderInfo(result);
		this.renderPager(result);
		if (this.config.onRender) { this.config.onRender(); }
	}

	renderRows(rows) {
		const body = $("table-body");
		body.replaceChildren();
		for (const row of rows) { body.append(this.buildRow(row)); }
	}

	buildRow(data) {
		const tr = document.createElement("tr");
		tr.className = "data-row";
		if (this.config.rowClass) {
			const extra = this.config.rowClass(data);
			if (extra) { tr.classList.add(extra); }
		}
		for (const text of this.config.cells(data)) { tr.append(this.cell(text)); }
		if (this.config.actions !== false) { tr.append(this.actionCell(data)); }
		tr.addEventListener("click", () => { location.href = this.config.url(data); });
		return tr;
	}

	cell(text) {
		const td = document.createElement("td");
		td.textContent = text;
		return td;
	}

	actionCell(data) {
		const td = document.createElement("td");
		td.className = "col-action";
		const link = document.createElement("a");
		link.href = this.config.url(data);
		link.className = "row-open";
		link.title = "Apri";
		link.innerHTML = ICON_OPEN;
		link.addEventListener("click", (event) => event.stopPropagation());
		td.append(link);
		return td;
	}

	renderInfo(result) {
		if (result.total === 0) { $("results-info").textContent = "Nessun risultato"; return; }
		const from = (result.page - 1) * result.page_size + 1;
		const to = from + result.rows.length - 1;
		$("results-info").textContent = "Risultati da " + from + " a " + to + " di " + formatNumber(result.total, ".");
	}

	renderPager(result) {
		const pager = $("pager");
		pager.replaceChildren();
		const last = Math.ceil(result.total / result.page_size);
		if (last <= 1) { return; }
		pager.append(this.pageButton("‹", result.page - 1, result.page > 1, false));
		for (const page of pageWindow(result.page, last)) {
			if (page === 0) { pager.append(gap()); continue; }
			pager.append(this.pageButton(String(page), page, true, page === result.page));
		}
		pager.append(this.pageButton("›", result.page + 1, result.page < last, false));
	}

	pageButton(label, page, enabled, current) {
		const button = document.createElement("button");
		button.type = "button";
		button.textContent = label;
		button.disabled = !enabled;
		if (current) { button.className = "current"; }
		if (enabled && !current) { button.addEventListener("click", () => this.goToPage(page)); }
		return button;
	}

	showSortState() {
		document.querySelectorAll("th[data-sort]").forEach((header) => this.markSort(header));
	}

	markSort(header) {
		header.classList.remove("sorted-asc", "sorted-desc");
		if (header.dataset.sort !== this.state.sort) { return; }
		header.classList.add(this.state.desc ? "sorted-desc" : "sorted-asc");
	}

	showError(message) {
		$("table-error").textContent = message;
	}

	saveState() {
		const saved = { sort: this.state.sort, desc: this.state.desc, page: this.state.page };
		for (const field in this.config.filters) { saved[field] = $(this.config.filters[field]).value; }
		try {
			sessionStorage.setItem(this.config.stateKey, JSON.stringify(saved));
		} catch (error) {
			// senza storage i filtri non si ricordano
		}
	}

	restoreState() {
		let saved;
		try {
			saved = JSON.parse(sessionStorage.getItem(this.config.stateKey));
		} catch (error) {
			return;
		}
		if (!saved) { return; }
		this.state.sort = saved.sort || this.state.sort;
		this.state.desc = saved.desc !== false;
		this.state.page = saved.page || 1;
		for (const field in this.config.filters) { $(this.config.filters[field]).value = saved[field] || ""; }
	}
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

function gap() {
	const span = document.createElement("span");
	span.textContent = "…";
	return span;
}