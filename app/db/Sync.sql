-- Stato della sincronizzazione cloud -> locale (solo locale).
-- SyncState: epoch del cloud, cursori per tabella ("cursor.patient", "cursor.visit"), "full_sync" (pending|done),
--            "last_sync_on" (UTC), "device_id".
-- SyncSeen: ID visti durante una full sync; a fine scarico si eliminano le righe locali non presenti sul cloud.
CREATE TABLE IF NOT EXISTS SyncState (
	Name TEXT PRIMARY KEY,
	Value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS SyncSeen (
	TableName TEXT NOT NULL,
	ID INTEGER NOT NULL,
	PRIMARY KEY (TableName, ID)
) WITHOUT ROWID;