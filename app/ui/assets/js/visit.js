// Scheda visita (sola lettura): visit.html?id=...

document.addEventListener("page-ready", onReady);

async function onReady() {
	const id = Number(new URLSearchParams(location.search).get("id"));
	if (!Number.isInteger(id) || id <= 0) { showError("Visita non valida."); return; }
	let visit;
	try {
		visit = await invoke("get_visit", { id: id });
	} catch (error) {
		showError(String(error));
		return;
	}
	const title = "Visita del " + formatDate(visit.date.slice(0, 10));
	$("visit-title").textContent = title;
	renderFields(visit);
	renderData(visit.data);
	addWordButton("visit-title", function () { return visitWord(visit, title); });
	addCloudButton("visit-title", "visit", visit.id);
	showPatientVisits(visit.patient_id, visit.id);
}

function visitWord(visit, title) {
	const fields = Array.from($("visit-fields").querySelectorAll("dt")).map(function (label) { return [label.textContent, label.nextElementSibling.textContent.trim()]; });
	const sections = [{ title: "Dati visita", rows: fields }];
	document.querySelectorAll("#visit-blocks .visit-block").forEach(function (block) { sections.push(visitWordSection(block)); });
	const patient = (visit.patient_surname + " " + visit.patient_name).trim();
	return {
		filename: "Visita " + visit.date.slice(0, 10).split("-").reverse().join("-") + (patient ? " " + patient : ""),
		title: "Riepilogo visita - " + title,
		sections: sections,
	};
}

function visitWordSection(block) {
	const heading = block.querySelector(".block-title").textContent.trim();
	const rows = [];
	block.querySelectorAll(":scope > .eyes > .eye").forEach(function (eye) {
		const eyeName = eye.querySelector(".eye-label").textContent.trim();
		const eyeRows = eye.querySelectorAll(":scope > .block-row");
		const text = eye.querySelector(":scope > .block-text");
		const table = eye.querySelector(":scope > table");
		eyeRows.forEach(function (row) { rows.push(wordRow(row, eyeName)); });
		if (text) { rows.push([eyeName, text.textContent.trim()]); }
		if (table) { addWordTableRows(rows, eyeName, table); }
		if (!eyeRows.length && !text && !table) { rows.push([eyeName, "-"]); }
	});
	block.querySelectorAll(":scope > .block-row").forEach(function (row) { rows.push(wordRow(row, "")); });
	const textFields = block.querySelector(":scope > .text-fields");
	if (textFields) {
		const children = Array.from(textFields.children);
		for (let index = 0; index < children.length; index += 2) { rows.push([children[index].textContent.trim(), children[index + 1].textContent.trim()]); }
	}
	return { title: heading, rows: rows };
}

function wordRow(row, prefix) {
	return [(prefix ? prefix + " - " : "") + row.children[0].textContent.trim(), row.children[1].textContent.trim()];
}

function addWordTableRows(rows, eyeName, table) {
	const tableRows = Array.from(table.rows);
	const headers = Array.from(tableRows.shift().cells).slice(1).map(function (cell) { return cell.textContent.trim(); });
	tableRows.forEach(function (row) {
		const cells = Array.from(row.cells).map(function (cell) { return cell.textContent.trim(); });
		const values = cells.slice(1).map(function (value, index) { return value ? headers[index] + ": " + value : ""; }).filter(Boolean);
		rows.push([eyeName + " - " + cells[0], values.join(" · ")]);
	});
}

function renderFields(visit) {
	const list = $("visit-fields");
	list.replaceChildren();
	addField(list, "ID visita", String(visit.id));
	addField(list, "Data", formatDate(visit.date.slice(0, 10)));
	addPatientField(list, visit);
	addField(list, "Motivo", visit.cause);
	addField(list, "Conclusioni", visit.diagnosis);
	addField(list, "Ultimo aggiornamento", formatDateTime(visit.last_update));
}

// Il paziente è un link alla sua scheda.
function addPatientField(list, visit) {
	const name = (visit.patient_surname + " " + visit.patient_name).trim() || "Paziente " + visit.patient_id;
	const link = element("a", "", name);
	link.href = "patient.html?id=" + encodeURIComponent(visit.patient_id);
	const field = element("div", "field", "");
	field.append(element("dt", "field-label", "Paziente"));
	const value = element("dd", "ro-input", "");
	value.append(link);
	field.append(value);
	list.append(field);
}

function addField(list, label, value) {
	if (!value) { return; }
	const field = element("div", "field", "");
	field.append(element("dt", "field-label", label));
	field.append(element("dd", "ro-input", value));
	list.append(field);
}

// I dati sono JSON; se non lo fossero si mostrano così come sono.
// ---- Dati della visita (VData) ----
// Struttura, etichette e opzioni sono in visit-schema.js; qui solo il disegno.

// Legge un valore: prima annidato (a.b.c), poi con il nome piatto "a>b>c".
function pick(data, path) {
	const parts = path.split(">");
	let node = data;
	for (const part of parts) {
		if (node === null || typeof node !== "object" || !(part in node)) { node = undefined; break; }
		node = node[part];
	}
	if (node === undefined && data && typeof data === "object" && path in data) { node = data[path]; }
	if (node === undefined || node === null || typeof node === "object") { return ""; }
	return decodeEntities(String(node)).trim();
}

// Nel cloud i testi sono salvati con entità HTML (es. &agrave;): si decodificano come testo, mai come HTML.
function decodeEntities(text) {
	if (text.indexOf("&") < 0) { return text; }
	const area = document.createElement("textarea");
	area.innerHTML = text;
	return area.value;
}

function optionLabel(value, optionsName) {
	if (!optionsName || !VISIT_OPTIONS[optionsName]) { return value; }
	return VISIT_OPTIONS[optionsName][value] || value;
}

function hasContent(value) {
	if (value === undefined || value === null) { return false; }
	if (typeof value === "object") { return Object.values(value).some(hasContent); }
	return String(value).trim() !== "";
}

function renderData(text) {
	const container = $("visit-blocks");
	container.replaceChildren();
	if (!text) { return; }
	let data;
	try {
		data = JSON.parse(text);
	} catch (error) {
		container.append(blockCard("Dati", [preText(text)]));
		return;
	}
	if (!data || typeof data !== "object") { return; }
	data = normalizeVisitData(data);

	const used = new Set(VISIT_HIDDEN_KEYS);
	VISIT_BLOCKS.forEach((block) => {
		collectKeys(block).forEach((key) => used.add(key.split(">")[0]));
		addNode(container, buildBlock(data, block));
	});

	// Qualsiasi chiave non prevista dallo schema: non si perde nulla.
	const others = Object.keys(data).filter((key) => !used.has(key) && hasContent(data[key]));
	if (others.length > 0) {
		const rows = others.map((key) => fieldRow(key, typeof data[key] === "object" ? JSON.stringify(data[key]) : decodeEntities(String(data[key]))));
		addNode(container, blockCard("Altri dati", rows));
	}
	$("visit-data").textContent = JSON.stringify(data, null, 2);
	$("visit-data-box").hidden = false;
}

// Chiavi di primo livello toccate da un blocco (per capire cosa resta in "Altri dati").
function collectKeys(block) {
	const keys = [];
	if (block.key) { keys.push(block.key); }
	if (block.keys) { block.keys.forEach((key) => keys.push(key)); }
	(block.fields || []).forEach(([key]) => { if (block.type === "single" || block.type === "texts") { keys.push(key); } });
	(block.extra || []).forEach(([key]) => keys.push(key));
	if (block.type === "ucva") { keys.push("ucva", "ucvan"); }
	if (block.type === "refraction") {
		VISIT_REFRACTION_ROWS.forEach(([key]) => keys.push(key));
		VISIT_REFRACTION_EXTRAS.forEach(([key]) => keys.push(key));
		VISIT_BINOCULAR.forEach(([key]) => keys.push(key));
	}
	return keys;
}

function buildBlock(data, block) {
	if (block.type === "eyes") { return eyesBlock(data, block); }
	if (block.type === "single") { return singleBlock(data, block); }
	if (block.type === "texts") { return textsBlock(data, block); }
	if (block.type === "ucva") { return ucvaBlock(data); }
	if (block.type === "refraction") { return refractionBlock(data); }
	return null;
}

function addNode(parent, node) { if (node) { parent.append(node); } }

function blockCard(title, children) {
	const card = element("section", "visit-block", "");
	card.append(element("h2", "section-title block-title", title));
	children.forEach((child) => card.append(child));
	return card;
}

function preText(value) { return element("p", "block-text", value); }

function fieldRow(label, value) {
	const row = element("div", "block-row", "");
	row.append(element("span", "block-label", label));
	row.append(element("span", "block-value", value));
	return row;
}

function extraRows(data, extras) {
	const rows = [];
	(extras || []).forEach(([key, label, options]) => {
		const value = pick(data, key);
		if (value) { rows.push(fieldRow(label, optionLabel(value, options))); }
	});
	return rows;
}

function eyeColumns(buildColumn) {
	const grid = element("div", "eyes", "");
	let any = false;
	VISIT_EYES.forEach(([key, label]) => {
		const column = element("div", "eye", "");
		column.append(element("div", "eye-label", label));
		if (buildColumn(column, key)) { any = true; }
		grid.append(column);
	});
	return any ? grid : null;
}

// Blocco a due colonne: OD a sinistra, OS a destra.
function eyesBlock(data, block) {
	const grid = eyeColumns((column, eye) => {
		const eyeIndex = eye === "r" ? 0 : 1;
		if (!block.fields) {
			const path = block.keys ? block.keys[eyeIndex] : block.key + ">" + eye;
			const value = pick(data, path);
			if (!value) { return false; }
			column.append(preText(optionLabel(value, block.options)));
			return true;
		}
		let filled = false;
		block.fields.forEach(([field, label, options]) => {
			const value = pick(data, block.key + ">" + eye + ">" + field);
			if (!value) { return; }
			column.append(fieldRow(label, optionLabel(value, options)));
			filled = true;
		});
		return filled;
	});
	const extras = extraRows(data, block.extra);
	if (!grid && extras.length === 0) { return null; }
	const children = [];
	if (grid) { children.push(grid); }
	extras.forEach((row) => children.push(row));
	return blockCard(block.title, children);
}

function singleBlock(data, block) {
	const rows = extraRows(data, block.fields);
	return rows.length > 0 ? blockCard(block.title, rows) : null;
}

function textsBlock(data, block) {
	const rows = block.fields.map(([key, label]) => [label, pick(data, key)]).filter(([, value]) => value);
	if (rows.length === 0) { return null; }
	const list = element("div", "text-fields", "");
	rows.forEach(([label, value]) => {
		list.append(element("div", "block-label", label));
		list.append(element("p", "block-text", value));
	});
	return blockCard(block.title, [list]);
}

function ucvaBlock(data) {
	const grid = eyeColumns((column, eye) => {
		const far = pick(data, "ucva>" + eye);
		const near = pick(data, "ucvan>" + eye);
		if (far) { column.append(fieldRow("UCVA lontano", far)); }
		if (near) { column.append(fieldRow("UCVA vicino", near)); }
		return !!(far || near);
	});
	return grid ? blockCard("Visus naturale", [grid]) : null;
}

// Una tabella per occhio: righe = lenti/BCVA/..., colonne = VIS S C A ADDI ADD (solo quelle con dati).
function refractionBlock(data) {
	const grid = eyeColumns((column, eye) => {
		const rows = VISIT_REFRACTION_ROWS.filter(([key]) => VISIT_REFRACTION_COLUMNS.some(([field]) => pick(data, key + ">" + eye + ">" + field)));
		if (rows.length === 0) { return false; }
		const columns = VISIT_REFRACTION_COLUMNS.filter(([field]) => rows.some(([key]) => pick(data, key + ">" + eye + ">" + field)));
		const table = element("table", "refraction", "");
		const head = element("tr", "", "");
		head.append(element("th", "", ""));
		columns.forEach(([, label]) => head.append(element("th", "", label)));
		table.append(head);
		rows.forEach(([key, label]) => {
			const line = element("tr", "", "");
			line.append(element("th", "", label));
			columns.forEach(([field]) => line.append(element("td", "", pick(data, key + ">" + eye + ">" + field))));
			table.append(line);
		});
		column.append(table);
		return true;
	});
	const children = [];
	if (grid) { children.push(grid); }
	extraRows(data, VISIT_BINOCULAR).forEach((row) => children.push(row));
	extraRows(data, VISIT_REFRACTION_EXTRAS).forEach((row) => children.push(row));
	return children.length > 0 ? blockCard("Refrazione", children) : null;
}

function showError(message) {
	$("visit-error").textContent = message;
}
