#!/usr/bin/env python3
"""Ripristina il DB locale da un backup (cartella backups/ accanto al DB).

Chiudere l'app prima di eseguirlo. Il DB attuale viene salvato come optigest_offline.db.prima-del-ripristino.
Dopo il ripristino la prima sincronizzazione recupera le modifiche successive al backup.

Uso:
	python3 scripts/restore_db.py --list
	python3 scripts/restore_db.py --latest --yes
	python3 scripts/restore_db.py --file optigest_offline-20261106-234500.db --yes
"""

import argparse
import glob
import os
import shutil
import sqlite3
import sys

DEFAULT_DIR = os.path.expanduser("~/Library/Application Support/com.eosmedtech.optigest.offline")
DB_NAME = "optigest_offline.db"


def main():
	args = parse_args()
	backups = sorted(glob.glob(os.path.join(args.dir, "backups", "optigest_offline-*.db")), reverse=True)
	if args.list or not (args.latest or args.file):
		for path in backups:
			print(os.path.basename(path), "%.1f MB" % (os.path.getsize(path) / 1e6))
		if not backups:
			print("Nessun backup trovato in", os.path.join(args.dir, "backups"))
		return
	source = backups[0] if args.latest and backups else os.path.join(args.dir, "backups", args.file or "")
	if not os.path.exists(source):
		sys.exit("Backup non trovato: " + source)
	if not args.yes:
		sys.exit("Operazione distruttiva: ripetere con --yes per ripristinare " + os.path.basename(source))
	check(source)
	target = os.path.join(args.dir, DB_NAME)
	if os.path.exists(target):
		shutil.copy2(target, target + ".prima-del-ripristino")
	for suffix in ("-wal", "-shm"):
		if os.path.exists(target + suffix):
			os.remove(target + suffix)
	shutil.copy2(source, target)
	print("Ripristinato:", os.path.basename(source))


def check(path):
	connection = sqlite3.connect("file:" + path + "?mode=ro", uri=True)
	try:
		result = connection.execute("PRAGMA integrity_check").fetchall()
	finally:
		connection.close()
	if result != [("ok",)]:
		sys.exit("Il backup non è integro: " + str(result[:3]))


def parse_args():
	parser = argparse.ArgumentParser(description="Ripristina il DB locale da un backup")
	parser.add_argument("--dir", default=DEFAULT_DIR, help="cartella dati dell'app")
	parser.add_argument("--list", action="store_true", help="elenca i backup")
	parser.add_argument("--latest", action="store_true", help="usa il backup più recente")
	parser.add_argument("--file", help="nome del backup da usare")
	parser.add_argument("--yes", action="store_true", help="conferma")
	return parser.parse_args()


if __name__ == "__main__":
	main()