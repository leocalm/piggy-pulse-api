-- Add major world currencies that were missing from the initial seed.
-- BRL, EUR, JPY already exist; skip any code that is already present.
INSERT INTO currency (name, symbol, currency, decimal_places, symbol_position)
SELECT name, symbol, currency, decimal_places, symbol_position
FROM (VALUES
    ('US Dollar',         '$',  'USD', 2, 'before'),
    ('British Pound',     '£',  'GBP', 2, 'before'),
    ('Canadian Dollar',   '$',  'CAD', 2, 'before'),
    ('Swiss Franc',       'Fr', 'CHF', 2, 'before'),
    ('Australian Dollar', '$',  'AUD', 2, 'before'),
    ('Swedish Krona',     'kr', 'SEK', 2, 'after'),
    ('Norwegian Krone',   'kr', 'NOK', 2, 'after'),
    ('Danish Krone',      'kr', 'DKK', 2, 'after'),
    ('Polish Złoty',      'zł', 'PLN', 2, 'after'),
    ('Czech Koruna',      'Kč', 'CZK', 2, 'after'),
    ('Hungarian Forint',  'Ft', 'HUF', 0, 'after')
) AS new(name, symbol, currency, decimal_places, symbol_position)
WHERE NOT EXISTS (
    SELECT 1 FROM currency c WHERE c.currency = new.currency
);
