ALTER TABLE settings
    ADD COLUMN dashboard_layout JSONB NOT NULL DEFAULT '{"widgetOrder":[],"hiddenWidgets":[]}';
