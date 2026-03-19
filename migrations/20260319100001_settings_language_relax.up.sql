-- Relax language check to allow any BCP 47 tag (v2 API validates in application code)
ALTER TABLE settings DROP CONSTRAINT IF EXISTS settings_language_check;
