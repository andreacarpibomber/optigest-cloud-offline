#!/usr/bin/env python3
"""Svuota il DB locale: pazienti, visite, documenti, eventi e stato della sincronizzazione.

Restano il registro eventi (Log; con --with-log si svuota anche quello) e l'id dell'installazione.
La sincronizzazione successiva sarà completa. Chiudere l'app prima di eseguirlo.

Uso:
	python3 scripts/reset_db.py --yes
	python3 scripts/reset_db.py --yes --with-log
	python3 scripts/reset_db.py --db /percorso/optigest_offline.db --yes
"""

import argparse
import os
import sqlite3
import sys

DEFAULT_DB = os.path.expanduser("~/Library/Application Support/com.eosmedtech.optigest.offline/optigest_offline.db")

WIPE_STATEMENTS = [
	"DELETE FROM Visit",
	"DELETE FROM Patient",
	"DELETE FROM Doc",
	"DELETE FROM Event",
	"DELETE FROM SyncSeen",
	"DELETE FROM SyncState WHERE Name <> 'device_id'",
]


def main():
	args = parse_args()
	if not os.path.exists(args.db):
		sys.exit("DB non trovato: " + args.db)
	if not args.yes:
		sys.exit("Operazione distruttiva: ripetere con --yes per confermare (DB: " + args.db + ")")

	connection = sqlite3.connect(args.db, timeout=5)
	try:
		wipe(connection, args.with_log)
	except sqlite3.OperationalError as error:
		sys.exit("Impossibile svuotare il DB (app aperta?): " + str(error))
	finally:
		connection.close()
	print("DB svuotato: " + args.db)


def wipe(connection, with_log):
	# secure_delete sovrascrive le pagine liberate; VACUUM ricostruisce il file
	connection.execute("PRAGMA secure_delete = ON")
	statements = list(WIPE_STATEMENTS)
	if with_log:
		statements.append("DELETE FROM Log")
	with connection:
		for statement in statements:
			connection.execute(statement)
	connection.execute("VACUUM")


def parse_args():
	parser = argparse.ArgumentParser(description="Svuota il DB locale di OptiGest Cloud Offline")
	parser.add_argument("--db", default=DEFAULT_DB, help="percorso del file SQLite")
	parser.add_argument("--yes", action="store_true", help="conferma lo svuotamento")
	parser.add_argument("--with-log", action="store_true", help="svuota anche il registro eventi")
	return parser.parse_args()


if __name__ == "__main__":
	main()