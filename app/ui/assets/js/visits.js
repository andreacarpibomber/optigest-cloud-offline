// Elenco visite. Clic sulla riga -> visit.html?id=... (logica della tabella in table.js)

new DataTable({
	command: "search_visits",
	stateKey: "optigest.visits",
	defaultSort: "id",
	filters: {
		id: "f-id",
		date: "f-date",
		surname: "f-surname",
		name: "f-name",
		birthday: "f-birthday",
		fiscal_code: "f-fiscal",
		cause: "f-cause",
		diagnosis: "f-diagnosis",
	},
	cells: function (visit) {
		return [
			String(visit.id),
			formatDate(visit.date.slice(0, 10)),
			visit.surname,
			visit.name,
			formatDate(visit.birthday),
			visit.fiscal_code,
			visit.cause,
			visit.diagnosis,
		];
	},
	url: function (visit) {
		return "visit.html?id=" + encodeURIComponent(visit.id);
	},
});