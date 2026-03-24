# API Gap Analysis — PiggyPulse Redesign

> Generated 2026-03-17. Compares the current OpenAPI v2 spec against all screens designed across 6 platforms (Desktop, Mobile Web, iOS, Android, iPad, Apple Watch).

## Current API Surface

The v2 API has **63 endpoints** across these domains:

| Domain | Endpoints | CRUD | Extras |
|--------|-----------|------|--------|
| Accounts | 11 | Full | summary, details, balance-history, adjust-balance, archive |
| Transactions | 4 | Full | direction/period/account/category/vendor filtering |
| Categories | 7 | Full | overview (spent/budgeted), options, archive |
| Targets | 3 | Create/Update | exclude |
| Vendors | 6 | Full | options, archive |
| Periods | 6 | Full | schedule (auto-generation) CRUD |
| Overlays | 7 | Full | transaction include/exclude, rules, caps |
| Dashboard | 3 | Read | current-period, net-position, budget-stability |
| Auth | 15 | — | login, register, 2FA (enable/verify/disable/emergency/backup), password flows |
| Settings | 8 | — | profile, preferences, sessions, export (CSV + JSON), delete account, reset |
| Onboarding | 2 | — | status, complete |
| Currencies | 2 | Read | list, detail |

---

## Gap Analysis

### Legend

- **EXISTS** — endpoint exists with the data the UI needs
- **PARTIAL** — endpoint exists but is missing fields or data the UI requires
- **MISSING** — no endpoint exists; must be created

---

### 1. Dashboard Cards

The redesign has 12 dashboard cards. The API only supports 3 of them directly.

| # | Card | Status | Current Support | Gap |
|---|------|--------|----------------|-----|
| 1 | Current Period | PARTIAL | `GET /dashboard/current-period` → spent, target, daysRemaining, daysInPeriod, projectedSpend | Missing **sparkline data** (daily spend time series). Remaining is computable client-side. |
| 2 | Net Position | PARTIAL | `GET /dashboard/net-position` → total, liquid, protected, debt, differenceThisPeriod | Missing **per-account balance list** inline. Must call `/accounts/summary` separately. |
| 3 | Budget Stability | PARTIAL | `GET /dashboard/budget-stability` → stability %, periodsWithinRange, periodsStability[] | Missing **recent vs all-time split** (e.g. last 3 periods vs all). |
| 4 | Variable Categories | PARTIAL | `GET /categories/overview` → per-category actual, budgeted, projected, variance | Missing **behavior field** to distinguish variable from fixed. Frontend can't filter without it. |
| 5 | Fixed Categories | MISSING | No concept of fixed vs variable category. No paid/partial/pending status. | Need `behavior` on Category + dedicated status endpoint. |
| 6 | Subscriptions | MISSING | No subscription entity at all. | Entire subscription domain needed. |
| 7 | Individual Account | PARTIAL | `/accounts/summary` has balance. `/accounts/{id}/balance-history` has chart data. | Works but requires N+1 calls. A **batch sparkline endpoint** would help. |
| 8 | Recent Transactions | EXISTS | `GET /transactions?limit=5&periodId=...` | Fully covered. |
| 9 | Cash Flow | MISSING | Per-account inflow/outflow exists on `/accounts/{id}/details`, but no aggregate. | Need `GET /dashboard/cash-flow`. |
| 10 | Spending Trend | MISSING | No cross-period spend data. | Need `GET /dashboard/spending-trend`. |
| 11 | Uncategorized Txns | MISSING | No null-category concept or filter. | Need `GET /dashboard/uncategorized` or a `categoryId=null` filter on transactions. |
| 12 | Top Vendors | MISSING | Vendor list has transaction count but no spend amounts. | Need `GET /dashboard/top-vendors`. |

### 2. Category Behavior

The biggest schema gap. Categories currently have `type` (direction: income/expense/transfer) but no behavior distinction.

| What | Status | Gap |
|------|--------|-----|
| `behavior` field (fixed/variable/subscription) | MISSING | Add to `CategoryBase` schema. Immutable after creation (like direction). |
| Auto-assigned colors | MISSING | Mapping: In-Fixed=#7CA8C4, In-Variable=#9AA0CC, In-Sub=#8B7EC8, Out-Fixed=#C48BA0, Out-Variable=#D4A0B6, Out-Sub=#B088A0 |
| Fixed category status (paid/partial/pending) | MISSING | Need `GET /dashboard/fixed-categories` returning status per fixed category per period. |
| Category detail page | PARTIAL | Overview has spent/budgeted per category, but no **cross-period trend** or **per-category stability dots**. |
| Category trend chart | MISSING | Need `GET /categories/{id}/trend` → spend per period for last N periods. |
| Category stability | MISSING | Need per-category stability data (within budget or not per period). Could be part of a detail endpoint. |

### 3. Subscriptions (New Domain)

Entirely new. The redesign treats subscriptions as a first-class entity separate from categories.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/subscriptions` | GET | List all subscriptions with billing info, status, next charge |
| `/subscriptions` | POST | Create subscription (linked to category + vendor, billing amount/cycle/day) |
| `/subscriptions/{id}` | GET | Detail with billing history |
| `/subscriptions/{id}` | PUT | Update subscription |
| `/subscriptions/{id}` | DELETE | Delete subscription |
| `/subscriptions/{id}/cancel` | POST | Mark as cancelled (not archive — different semantics) |
| `/subscriptions/upcoming` | GET | Timeline of next N charges across all subscriptions |

**Schema: `SubscriptionResponse`**
```
id, name, categoryId, vendorId, billingAmount, billingCycle (monthly/yearly/weekly),
billingDay, nextChargeDate, status (active/cancelled/paused),
cancelledAt, createdAt, updatedAt
```

**Schema: `SubscriptionBillingEvent`** (for billing history)
```
id, subscriptionId, transactionId, amount, date, detected (boolean — auto-matched vs manual)
```

**Feature: Post-cancellation charge detection** — when a transaction matches a cancelled subscription's vendor + amount pattern, flag it in the response.

### 4. Vendor Analytics

Vendor detail page and merge flow require new endpoints.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/vendors/{id}/detail` | GET | Period spend, tx count, average, trend chart data, top categories, transactions |
| `/vendors/{id}/merge` | POST | Body: `{ targetVendorId }`. Reassign all transactions to target, delete source. |
| `/vendors/stats` | GET | Total vendors, total spend this period, avg per vendor |

The current `VendorSummaryResponse` has `numberOfTransactions` but no `totalSpend`. Either add `totalSpend` to the existing response or create the new detail endpoint.

### 5. Account Type-Specific Fields

Account detail pages for Allowance and Credit Card show type-specific data the API doesn't provide.

| Account Type | Missing Fields |
|-------------|----------------|
| Allowance | `topUpAmount`, `topUpCycle` (weekly/biweekly/monthly), `topUpDay` |
| Credit Card | `statementCloseDay`, `paymentDueDay` |

These should be added as **optional fields** on `AccountResponse` and `CreateAccountRequest`/`UpdateAccountRequest`, present only when the account type matches.

**Note:** Available credit (`spendLimit - currentBalance`) and utilization (`currentBalance / spendLimit`) are computable client-side from existing fields.

### 6. Overlay Category Breakdown

The overlay detail page shows per-category spent amounts within the overlay, but the API only returns caps.

| What | Status | Gap |
|------|--------|-----|
| `OverlayResponse.categoryCaps` | EXISTS | Has categoryId + capAmount per cap |
| Per-category **spent** within overlay | MISSING | Need `categoryBreakdown` on `OverlayResponse`: `[{ categoryId, categoryName, spentAmount }]` |

### 7. Transactions Batch

Quick Add (batch entry) creates multiple transactions at once.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/transactions/batch` | POST | Body: array of `CreateTransactionRequest`. Returns array of created transactions. |

### 8. Onboarding Enhancements

| What | Status | Gap |
|------|--------|-----|
| `OnboardingStep` enum | PARTIAL | Has `period, accounts, categories, summary`. Missing **`currency`** step. |
| Category templates | MISSING | Need `GET /onboarding/category-templates` returning predefined template definitions (Essential 5, Detailed 12). |
| Apply template | MISSING | Need `POST /onboarding/apply-template` accepting template ID, creating all categories at once. |

### 9. Settings Extensions

| What | Status | Gap |
|------|--------|-----|
| Dashboard layout | MISSING | Add `dashboardLayout` to `PreferencesResponse`: `{ widgetOrder: string[], hiddenWidgets: string[] }` |
| Compact mode | MISSING | Add `compactMode: boolean` to `PreferencesResponse` |
| Data import | MISSING | Need `POST /settings/import/data` accepting the same JSON format as export |

### 10. Period Gap Detection

The dashboard gap state shows when there's no active period covering today.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/periods/gaps` | GET | Returns date ranges not covered by any period |

**Alternative:** Add `gapBefore: { startDate, endDate } | null` to `PeriodResponse` so the gap info comes inline with the period list.

---

## Implementation Task List

### Phase 1 — Schema Changes (unblocks most UI)

- [ ] **1.1** Add `behavior` enum (`fixed`, `variable`, `subscription`) to `CategoryBase` schema
- [ ] **1.2** Add `behavior` to `CreateCategoryRequest` (required) and migration to backfill existing categories
- [ ] **1.3** Add optional `topUpAmount`, `topUpCycle`, `topUpDay` fields to `AccountResponse` / `CreateAccountRequest` / `UpdateAccountRequest`
- [ ] **1.4** Add optional `statementCloseDay`, `paymentDueDay` fields to Account (Credit Card type)
- [ ] **1.5** Add `currency` to `OnboardingStep` enum
- [ ] **1.6** Add `dashboardLayout` and `compactMode` to `PreferencesResponse` / `UpdatePreferencesRequest`
- [ ] **1.7** Add `categoryBreakdown` (array of `{ categoryId, categoryName, spentAmount }`) to `OverlayResponse`

### Phase 2 — Dashboard Endpoints

- [ ] **2.1** `GET /dashboard/current-period` — add `dailySpend: number[]` array for sparkline
- [ ] **2.2** `GET /dashboard/budget-stability` — add `recentStability` (last 3 periods) alongside existing all-time
- [ ] **2.3** `GET /dashboard/cash-flow` — new endpoint: `{ inflows, outflows, net }` for current period
- [ ] **2.4** `GET /dashboard/spending-trend` — new endpoint: `[{ periodId, periodName, totalSpend }]` for last N periods
- [ ] **2.5** `GET /dashboard/top-vendors` — new endpoint: `[{ vendorId, vendorName, totalSpend, percentage }]`
- [ ] **2.6** `GET /dashboard/uncategorized` — new endpoint: `{ count, transactions[] }` (transactions with no category)
- [ ] **2.7** `GET /dashboard/fixed-categories` — new endpoint: fixed category status (`paid`/`partial`/`pending`) per category for current period

### Phase 3 — Subscriptions Domain (New)

- [ ] **3.1** Design `subscriptions` database table + migration
- [ ] **3.2** Design `subscription_billing_events` table for billing history
- [ ] **3.3** `SubscriptionResponse` schema
- [ ] **3.4** `GET /subscriptions` — list with filtering (status, upcoming)
- [ ] **3.5** `POST /subscriptions` — create
- [ ] **3.6** `GET /subscriptions/{id}` — detail with billing history
- [ ] **3.7** `PUT /subscriptions/{id}` — update
- [ ] **3.8** `DELETE /subscriptions/{id}` — delete
- [ ] **3.9** `POST /subscriptions/{id}/cancel` — mark cancelled
- [ ] **3.10** `GET /subscriptions/upcoming` — next N charges timeline
- [ ] **3.11** Post-cancellation charge detection logic (match transaction against cancelled subscription patterns)

### Phase 4 — Vendor Analytics

- [ ] **4.1** `GET /vendors/{id}/detail` — period spend, tx count, average, trend, top categories, recent transactions
- [ ] **4.2** `POST /vendors/{id}/merge` — reassign transactions + delete source vendor
- [ ] **4.3** `GET /vendors/stats` — aggregate stats (total vendors, total spend, average)
- [ ] **4.4** Add `totalSpend` to `VendorSummaryResponse` (or deprecate in favor of detail endpoint)

### Phase 5 — Category Detail

- [ ] **5.1** `GET /categories/{id}/detail` — spent vs budget, trend data, stability dots, period transactions
- [ ] **5.2** `GET /categories/{id}/trend` — spend per period for last N periods (chart data)

### Phase 6 — Batch Operations & Import

- [ ] **6.1** `POST /transactions/batch` — create multiple transactions at once
- [ ] **6.2** `POST /settings/import/data` — import JSON backup (reverse of export)

### Phase 7 — Onboarding & Templates

- [ ] **7.1** Define category templates (Essential 5, Detailed 12) as static data
- [ ] **7.2** `GET /onboarding/category-templates` — list available templates with category definitions
- [ ] **7.3** `POST /onboarding/apply-template` — bulk-create categories from template

### Phase 8 — Period Gaps

- [ ] **8.1** `GET /periods/gaps` — return uncovered date ranges between periods
- [ ] **8.2** Alternatively: add `gapBefore` to `PeriodResponse` for inline gap detection

---

## Endpoint Summary

| Category | New Endpoints | Modified Endpoints | Schema Changes |
|----------|--------------|-------------------|----------------|
| Dashboard | 5 new | 2 modified | — |
| Subscriptions | 7 new | — | 2 new tables, 2 new schemas |
| Vendors | 3 new | — | Add totalSpend to summary |
| Categories | 2 new | — | Add behavior field |
| Transactions | 1 new | — | — |
| Accounts | — | — | Add allowance + CC fields |
| Overlays | — | — | Add categoryBreakdown |
| Settings | 1 new | 1 modified | Add dashboardLayout, compactMode |
| Onboarding | 2 new | — | Add currency step |
| Periods | 1 new | — | — |
| **Total** | **22 new** | **3 modified** | **7 schema changes** |

---

## Notes

- All new endpoints follow existing patterns: session cookie auth, keyset pagination, snake_case JSON, scoped to current user.
- Breaking schema changes (adding required fields like `behavior`) need a migration strategy — suggest making `behavior` optional initially with a default, then backfilling.
- The subscription domain is the largest piece of new work. Consider implementing it as a thin wrapper around categories with `behavior: subscription` + vendor linkage, rather than a fully separate entity, to reduce duplication.
- Dashboard endpoints should accept an optional `periodId` parameter (defaults to current period) for consistency with existing patterns.
