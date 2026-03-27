ALTER TABLE settings
    ADD COLUMN color_theme TEXT NOT NULL DEFAULT 'nebula'
        CHECK (color_theme IN ('nebula', 'sunrise', 'sage_stone', 'deep_ocean', 'warm_rose', 'moonlit'));
