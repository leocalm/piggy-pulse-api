-- Re-add language check constraint
ALTER TABLE settings ADD CONSTRAINT settings_language_check CHECK (language IN ('en', 'es', 'pt', 'fr', 'de'));
