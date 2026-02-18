ALTER TABLE settings
DROP CONSTRAINT IF EXISTS settings_budget_stability_tolerance_basis_points_check;

ALTER TABLE settings
DROP COLUMN IF EXISTS budget_stability_tolerance_basis_points;
