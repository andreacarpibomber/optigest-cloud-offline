// Elenco pazienti. Clic sulla riga -> patient.html?id=... (logica della tabella in table.js)

new DataTable({
	command: "search_patients",
	stateKey: "optigest.patients",
	defaultSort: "id",
	filters: {
		id: "f-id",
		surname: "f-surname",
		name: "f-name",
		birthday: "f-birthday",
		fiscal_code: "f-fiscal",
	},
	cells: function (patient) {
		return [String(patient.id), patient.surname, patient.name, formatDate(patient.birthday), patient.fiscal_code];
	},
	url: function (patient) {
		return "patient.html?id=" + encodeURIComponent(patient.id);
	},
});