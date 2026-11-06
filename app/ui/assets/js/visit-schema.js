// Schema unico dei campi della visita (VData), allineato a visit.php / suggestions del cloud.
// Un solo posto per etichette, ordine e opzioni: visit.js lo legge e basta.
// Percorsi: "sx>r" = data.sx.r (il cloud usa ">" come separatore).

const VISIT_EYES = [["r", "OD"], ["l", "OS"]];

const VISIT_OPTIONS = {
	ishihara: { rm: "Protanopia", ra: "Protanomalia", bm: "Deuteranopia", ba: "Deuteranomalia", "+": "Anomala percezione dei colori aspecifica", und: "Non valutabile", "-": "Nella norma" },
	dominantEye: { dx: "OD", sx: "OS", r: "OD", l: "OS" },
	mires: { r: "Regolari", i: "Irregolari" },
	prismBase: { i: "Interna", e: "Esterna", a: "Alta", b: "Bassa" }
};

// Colonne della tabella di refrazione (solo quelle con dati vengono mostrate)
const VISIT_REFRACTION_COLUMNS = [["vis", "VIS"], ["s", "S"], ["c", "C"], ["a", "A"], ["addi", "ADDI"], ["add", "ADD"]];

// Righe di refrazione: chiave VData, etichetta
const VISIT_REFRACTION_ROWS = [
	["l_inuse", "Lenti in uso"],
	["l_inusei", "Lenti in uso INT"],
	["l_inusen", "Lenti in uso PV"],
	["bcva", "BCVA"],
	["bcvai", "BCVA INT"],
	["bcvan", "BCVA PV"],
	["l_pr", "Lenti prescritte"],
	["l_pri", "Lenti prescritte INT"],
	["l_prn", "Lenti prescritte PV"],
	["lac_inuse", "LAC in uso"],
	["lac_pre", "LAC prescritte"],
	["r_din", "Refrazione dinamica"],
	["r_stat", "Refrazione statica"]
];

// Valori singoli legati alla refrazione, mostrati sotto la tabella: [chiave, etichetta, opzioni]
const VISIT_REFRACTION_EXTRAS = [
	["l_inuse>type", "Tipologia lenti in uso"],
	["l_inuse>foct", "Lenti in uso (focale/tipo)"],
	["lac_inuse>type", "Tipologia LAC in uso"],
	["lac_inuse>foct", "LAC in uso (focale/tipo)"],
	["lac_pre>type", "Tipologia LAC prescritte"],
	["lac_pre>foct", "LAC prescritte (focale/tipo)"],
	["l_pr>foct", "Lenti prescritte (focale/tipo)"],
	["r_stat>d", "Farmaco cicloplegia"],
	["pd_inuse", "PD lenti in uso (mm)"],
	["pdn_inuse", "PD vicino lenti in uso (mm)"],
	["pd_bcva", "PD BCVA (mm)"],
	["pdn_bcva", "PD vicino BCVA (mm)"],
	["pd_pr", "PD prescrizione (mm)"],
	["pdn_pr", "PD vicino prescrizione (mm)"],
	["pd", "PD (mm)"],
	["pdn", "PD vicino (mm)"]
];

// Valori binoculari (occhio "o"): [chiave, etichetta]
const VISIT_BINOCULAR = [
	["ucva>o", "UCVA lontano OO"],
	["ucvan>o", "UCVA vicino OO"],
	["l_inuse>o>vis", "Lenti in uso OO"],
	["bcva>o>vis", "BCVA OO"],
	["l_pr>o>vis", "Lenti prescritte OO"],
	["l_prn>o>vis", "Lenti prescritte PV OO"]
];

// Blocchi in ordine di visualizzazione.
//   type "eyes":   una colonna per occhio; "key" (testo libero) oppure "fields" (sottocampi [chiave, etichetta, opzioni])
//   keys: coppia esplicita [OD, OS] quando i nomi non seguono "chiave>r|l" (es. amsler_r / amsler_l)
//   extra: valori singoli sotto il blocco [chiave, etichetta, opzioni]
const VISIT_BLOCKS = [
	{ type: "eyes", title: "Sintomatologia", key: "sx" },
	{ type: "ucva" },
	{ type: "refraction" },
	{ type: "eyes", title: "Lenti prismatiche prescritte", key: "lp", fields: [["d", "Diottrie"], ["b", "Base", "prismBase"], ["a", "Asse"]] },
	{ type: "eyes", title: "Lenti prismatiche in uso", key: "lp_inuse", fields: [["d", "Diottrie"], ["b", "Base", "prismBase"], ["a", "Asse"]] },
	{ type: "single", title: "Occhio dominante", fields: [["de", "Occhio dominante", "dominantEye"]] },
	{ type: "eyes", title: "Tonometria", key: "tono", fields: [["m", "Tono misurato"], ["p", "Pachimetria"], ["c", "Tono corretto"]], extra: [["tono_meas_type", "Tipologia misura"], ["tono_meas_time", "Orario misura"]] },
	{ type: "eyes", title: "Cheratometria", key: "kr", fields: [["k1", "K1"], ["k2", "K2"], ["km", "K medio"], ["kd", "K2-K1"], ["ak1", "Asse K1"], ["ak2", "Asse K2"], ["mir", "Mire", "mires"]] },
	{ type: "eyes", title: "Ortottica", key: "ox" },
	{ type: "eyes", title: "Annessi", key: "ax" },
	{ type: "eyes", title: "Segmento anteriore", key: "anx" },
	{ type: "eyes", title: "Segmento posteriore", key: "pox" },
	{ type: "eyes", title: "Iride", key: "iris" },
	{ type: "eyes", title: "Camera anteriore", key: "caa" },
	{ type: "eyes", title: "Cristallino", key: "crys" },
	{ type: "eyes", title: "Corpo vitreo", key: "vitr" },
	{ type: "eyes", title: "Opacità cristallino", key: "opac" },
	{ type: "eyes", title: "Pupilla", key: "pup", fields: [["v", "Pupilla"], ["scot", "Scotopica"], ["midr", "Midriasi"], ["psur", "Post-chirurgia"]] },
	{ type: "eyes", title: "Schiascopia", key: "sch" },
	{ type: "eyes", title: "Test di Ishihara", keys: ["ishihara_r", "ishihara_l"], options: "ishihara" },
	{ type: "eyes", title: "Test di Amsler", keys: ["amsler_r", "amsler_l"] },
	{ type: "eyes", title: "Test di Lang", keys: ["lang_r", "lang_l"], extra: [["lang", "Lang"]] },
	{ type: "single", title: "Altri test", fields: [["sfly", "Stereo Fly"], ["worth", "Worth"]] },
	{ type: "eyes", title: "OCT", key: "oct" },
	{ type: "eyes", title: "Campo visivo", key: "fov" },
	{ type: "eyes", title: "Mappa corneale", key: "cornmap" },
	{ type: "eyes", title: "Topografia corneale", key: "corn_topog" },
	{ type: "eyes", title: "Aberrometria", key: "aberr", fields: [["osa", "OSA"], ["malacr", "Malacr"], ["type", "Tipo"]] },
	{ type: "eyes", title: "IOL", key: "iol", fields: [["form", "Formula"], ["ecob", "Eco B"], ["acd", "ACD"], ["costa", "Costante"], ["pow", "Potere"], ["exprf", "Refrazione attesa"], ["iol", "IOL"]] },
	{ type: "eyes", title: "Intervento", key: "surg", fields: [["st_d", "Data"], ["st_t", "Ora"], ["surgn", "Chirurgo"]], extra: [["filter_eye", "Occhio"], ["sur", "Intervento"], ["suri", "Info intervento"]] },
	{ type: "eyes", title: "Pre-intervento", key: "psurg", fields: [["st_d", "Data"], ["st_t", "Ora"], ["surgn", "Chirurgo"], ["ans", "Anestesia"], ["center", "Centro"], ["nx", "Note"]] },
	{ type: "eyes", title: "Chirurgia refrattiva laser", key: "pschir", fields: [["s", "S"], ["c", "C"], ["a", "A"], ["eg", "EG"], ["tr", "TR"], ["end", "Endotelio"], ["axl", "AXL"], ["acd", "ACD"], ["ww", "WW"], ["pu_f", "Pupilla fot."], ["pu_m", "Pupilla mes."], ["pu_s", "Pupilla scot."], ["flap_t", "Flap spessore"], ["flap_d", "Flap diametro"]] },
	{ type: "eyes", title: "Laser", key: "ps", fields: [["ltype", "Tipologia"], ["ltecn", "Tecnica"], ["mcher", "Microcheratomo"], ["suzring", "Anello di suzione"], ["diam", "Diametro"]] },
	{ type: "eyes", title: "IOL fachica", key: "pschiriol", fields: [["mod", "Modello"], ["pow", "Potere"], ["axin", "Asse impianto"], ["axor", "Asse orientamento"], ["tr", "TR"]] },
	{ type: "eyes", title: "Chirurgia cataratta", key: "pschircat", fields: [["brand", "Marca"], ["mod", "Modello"], ["pow", "Potere"], ["axcil", "Asse cilindro"], ["axin", "Asse impianto"], ["tr", "TR"]] },
	{ type: "texts", title: "Dettagli visita", fields: [
		["cx", "Motivo visita"], ["ex", "Esami eseguiti"], ["dx", "Conclusioni"], ["px", "Esami prescritti"],
		["tx", "Terapia"], ["tsx", "Terapia chirurgica"], ["rx", "Note prescrizione lenti"], ["nx", "Note aggiuntive"],
		["ins", "Assicurazione / Solvenza"], ["vhs", "Prestazioni"]
	] },
	{ type: "texts", title: "Note private", fields: [["npx", "Note private visita"]] }
];

// Chiavi presenti in VData ma volutamente non mostrate
const VISIT_HIDDEN_KEYS = ["edby", "k", "visit_date", "visit_time", "VOwnerID", "ttx"];

// Allineamento Hub/Cloud (come in visit.php): il vecchio formato "k" diventa "kr".
const VISIT_LEGACY_KERATOMETRY = [["1", "k1"], ["2", "k2"], ["@1", "ak1"], ["@2", "ak2"], ["m", "mir"], ["1/2", "km"], ["c", "kd"]];

function normalizeVisitData(data) {
	if (!data || typeof data !== "object" || !data.k || typeof data.k !== "object") { return data; }
	data.kr = data.kr || {};
	VISIT_EYES.forEach(([eye]) => {
		if (!data.k[eye]) { return; }
		data.kr[eye] = data.kr[eye] || {};
		VISIT_LEGACY_KERATOMETRY.forEach(([from, to]) => {
			if (data.k[eye][from] !== undefined) { data.kr[eye][to] = data.k[eye][from]; }
		});
	});
	return data;
}