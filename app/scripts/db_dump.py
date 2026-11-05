#!/usr/bin/env python3
"""Popola il DB locale con dati casuali di prova: 4 pazienti e 6 visite (default).

Uso:
	python3 scripts/seed.py                 # aggiunge 4 pazienti e 6 visite
	python3 scripts/seed.py --reset         # elimina prima i dati di prova precedenti
	python3 scripts/seed.py --with-deleted  # aggiunge anche 1 paziente e 1 visita eliminati (non devono comparire nei conteggi)
	python3 scripts/seed.py --db /percorso/optigest_offline.db

I dati di prova sono riconoscibili da PatientInfo = 'SEED': --reset toglie solo quelli.
Il DB e le tabelle vengono creati dall'app al primo avvio (make run).
"""

import argparse
import hashlib
import json
import os
import random
import sqlite3
import sys
from datetime import datetime, timedelta, timezone

DEFAULT_DB = os.path.expanduser("~/Library/Application Support/com.eosmedtech.optigest.offline/optigest_offline.db")
SEED_MARK = "SEED"

NAMES_M = ["Marco", "Luca", "Giovanni", "Andrea", "Paolo", "Matteo"]
NAMES_F = ["Giulia", "Chiara", "Sara", "Elena", "Francesca", "Anna"]
SURNAMES = ["Rossi", "Bianchi", "Ferrari", "Esposito", "Romano", "Colombo", "Ricci", "Marino"]
CITIES = [("Milano", "MI", "20121"), ("Roma", "RM", "00184"), ("Torino", "TO", "10121"), ("Bologna", "BO", "40121"), ("Lugano", "TI", "6900")]
CAUSES = ["Controllo", "Calo della vista", "Occhio rosso", "Prima visita", "Prescrizione lenti", "Bruciore"]
DIAGNOSES = ["Miopia", "Ipermetropia", "Astigmatismo", "Congiuntivite", "Presbiopia", "Normale"]


def main():
	args = parse_args()
	if not os.path.exists(args.db):
		sys.exit("DB non trovato: " + args.db + "\nAvvia prima l'app (make run) per crearlo, oppure usa --db.")
	connection = sqlite3.connect(args.db)
	try:
		check_schema(connection)
		with connection:
			if args.reset:
				reset(connection)
			patient_ids = [insert_patient(connection, 0) for _ in range(args.patients)]
			for index in range(args.visits):
				insert_visit(connection, patient_ids[index % len(patient_ids)], 0)
			if args.with_deleted:
				deleted_id = insert_patient(connection, 1)
				insert_visit(connection, deleted_id, 1)
		print_totals(connection, args)
	finally:
		connection.close()


def parse_args():
	parser = argparse.ArgumentParser(description="Popola il DB locale con pazienti e visite casuali.")
	parser.add_argument("--db", default=DEFAULT_DB, help="percorso del DB (default: quello dell'app su macOS)")
	parser.add_argument("--patients", type=int, default=4)
	parser.add_argument("--visits", type=int, default=6)
	parser.add_argument("--reset", action="store_true", help="elimina prima i dati di prova precedenti")
	parser.add_argument("--with-deleted", action="store_true", help="aggiunge anche un paziente e una visita eliminati")
	args = parser.parse_args()
	if args.patients < 1 or args.visits < 0:
		parser.error("servono almeno 1 paziente e 0 o più visite")
	return args


def check_schema(connection):
	names = {row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
	if "Patient" not in names or "Visit" not in names:
		sys.exit("Le tabelle Patient/Visit non esistono: avvia l'app (make run) per installare lo schema.")


def reset(connection):
	connection.execute("DELETE FROM Visit WHERE VPatientID IN (SELECT PatientID FROM Patient WHERE PatientInfo = ?)", (SEED_MARK,))
	connection.execute("DELETE FROM Patient WHERE PatientInfo = ?", (SEED_MARK,))


def insert_patient(connection, deleted):
	patient_id = next_id(connection, "Patient", "PatientID")
	female = random.random() < 0.5
	name = random.choice(NAMES_F if female else NAMES_M)
	surname = random.choice(SURNAMES)
	city, province, postal_code = random.choice(CITIES)
	birthday = random_date(1940, 2015)
	now = now_text()
	connection.execute(
		"""INSERT INTO Patient (
			PatientID, PatientName, PatientSurname, PatientBirthday, PatientBirthPlace, PatientBirthProvince,
			PatientGender, PatientFiscalCode, PatientSendingPracticioner, PatientCurrentPractitioner, PatientEmail,
			PatientTags, PatientAllergies, PatientGeneralAnamnesy, PatientOcularAnamnesy, PatientAddress,
			PatientCity, PatientProvince, PatientPostalCode, PatientNation, PatientInfo, PatientOptions,
			PatientTelephones, PatientLastConsentDate, PatientCreatedOn, PatientLastUpdate, PatientDeleted
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
		(
			patient_id, name, surname, birthday, city, province,
			"F" if female else "M", fake_fiscal_code(), "", "", (name + "." + surname).lower() + "@example.com",
			"", "", "", "", "Via Roma " + str(random.randint(1, 120)),
			city, province, postal_code, "IT", SEED_MARK, json.dumps({}),
			json.dumps(["+39 3" + str(random.randint(10, 99)) + " " + str(random.randint(1000000, 9999999))]),
			None, now, now, deleted,
		),
	)
	return patient_id


def insert_visit(connection, patient_id, deleted):
	visit_id = next_id(connection, "Visit", "VisitID")
	data = json.dumps({
		"od": {"sph": random_diopter(), "cyl": random_diopter(), "ax": random.randint(0, 180)},
		"os": {"sph": random_diopter(), "cyl": random_diopter(), "ax": random.randint(0, 180)},
		"note": "Dato di prova",
	})
	connection.execute(
		"""INSERT INTO Visit (VisitID, VPatientID, VDate, VCause, VDiagnosis, VData, VChecksum, VLastUpdate, VDeleted)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
		(
			visit_id, patient_id, random_date(2023, 2026), random.choice(CAUSES), random.choice(DIAGNOSES),
			data, hashlib.md5(data.encode()).hexdigest(), now_text(), deleted,
		),
	)
	return visit_id


def print_totals(connection, args):
	patients = connection.execute("SELECT COUNT(*) FROM Patient WHERE PatientDeleted = 0").fetchone()[0]
	visits = connection.execute("SELECT COUNT(*) FROM Visit WHERE VDeleted = 0").fetchone()[0]
	print("Inseriti " + str(args.patients) + " pazienti e " + str(args.visits) + " visite.")
	print("Totali non eliminati nel DB: " + str(patients) + " pazienti, " + str(visits) + " visite.")


def next_id(connection, table, column):
	return connection.execute("SELECT COALESCE(MAX(" + column + "), 0) + 1 FROM " + table).fetchone()[0]


def random_date(year_from, year_to):
	start = datetime(year_from, 1, 1)
	span_days = (min(datetime(year_to, 12, 31), datetime.now()) - start).days
	return (start + timedelta(days=random.randint(0, span_days))).strftime("%Y-%m-%d")


def random_diopter():
	return round(random.randint(-12, 8) * 0.25, 2)


def fake_fiscal_code():
	letters = "ABCDEFGHILMNOPRSTUVZ"
	return "".join(random.choice(letters) for _ in range(6)) + "".join(str(random.randint(0, 9)) for _ in range(2)) + random.choice(letters) + "".join(str(random.randint(0, 9)) for _ in range(2)) + random.choice(letters) + "".join(str(random.randint(0, 9)) for _ in range(3)) + random.choice(letters)


def now_text():
	return datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S")


main()