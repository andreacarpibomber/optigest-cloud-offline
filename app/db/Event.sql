-- Appuntamenti (da Event).
CREATE TABLE IF NOT EXISTS Event (
	EventID INTEGER PRIMARY KEY,
	EventOwnerID INTEGER,
	EventLastEditorID INTEGER NOT NULL,
	EventOrgID INTEGER NOT NULL,
	EventTitle TEXT NOT NULL,
	EventStart TEXT NOT NULL,
	EventEnd TEXT NOT NULL,
	EventData TEXT NOT NULL CHECK (json_valid(EventData)),
	EventLastEdit TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
	EventDeleted INTEGER NOT NULL DEFAULT 0,
	GCalID TEXT GENERATED ALWAYS AS (
		CASE WHEN EventDeleted = 0 THEN json_extract(EventData, '$.gcalid') ELSE NULL END
	) STORED
);
CREATE INDEX IF NOT EXISTS idx_Event_org_deleted ON Event (EventOrgID, EventDeleted);
CREATE INDEX IF NOT EXISTS idx_Event_org_deleted_start ON Event (EventOrgID, EventDeleted, EventStart);
CREATE UNIQUE INDEX IF NOT EXISTS uq_Event_owner_org_gcalid ON Event (EventOwnerID, EventOrgID, GCalID);
