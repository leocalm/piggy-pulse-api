ALTER TABLE settings
ADD COLUMN IF NOT EXISTS budget_stability_tolerance_basis_points INTEGER NOT NULL DEFAULT 1000;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'settings_budget_stability_tolerance_basis_points_check'
    ) THEN
        ALTER TABLE settings
        ADD CONSTRAINT settings_budget_stability_tolerance_basis_points_check
        CHECK (budget_stability_tolerance_basis_points >= 0 AND budget_stability_tolerance_basis_points <= 10000);
    END IF;
END $$;
