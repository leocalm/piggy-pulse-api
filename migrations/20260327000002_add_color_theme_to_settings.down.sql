-- NOTE: Dropping these columns is lossy — any color_theme and related values stored for users will be permanently lost.
ALTER TABLE settings DROP COLUMN color_theme;
