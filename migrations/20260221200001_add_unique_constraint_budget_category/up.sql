CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_category_user_category
ON budget_category (user_id, category_id);
