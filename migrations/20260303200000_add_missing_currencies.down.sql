DELETE FROM currency
WHERE currency IN ('USD','GBP','CAD','CHF','AUD','SEK','NOK','DKK','PLN','CZK','HUF')
  AND user_id IS NULL;
