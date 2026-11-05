-- Visite (da _ORG_PREFIX_Visit).
CREATE TABLE IF NOT EXISTS Visit (
	VisitID INTEGER PRIMARY KEY,
	VPatientID INTEGER NOT NULL,
	VDate TEXT NOT NULL,
	VCause TEXT NOT NULL,
	VDiagnosis TEXT NOT NULL,
	VData TEXT NOT NULL CHECK (json_valid(VData)),
	VChecksum TEXT NOT NULL,
	VLastUpdate TEXT NOT NULL,
	VDeleted INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_Visit_deleted ON Visit (VDeleted);
-- Sostituisce idx_patient_deleted del dump, che è un prefisso di questo indice.
CREATE INDEX IF NOT EXISTS idx_Visit_patient_deleted_date ON Visit (VPatientID, VDeleted, VDate);
