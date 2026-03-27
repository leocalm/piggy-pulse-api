-- NOTE: Dropping this column is lossy — any avatar values stored for users will be permanently lost.
ALTER TABLE users DROP COLUMN avatar;
