# snop_cockpit_be

The S&OP Cockpit's **own-database layer** — an actix-web + MariaDB service that holds the planning
data **ERPNext is _not_ the source of truth for**. The cockpit frontend (`cockpit-next`) reads ERP
for facts (stock, sales, POs, lead time) and reads *this* service for decisions (service levels,
MOQ, order type, EOQ costs) and audit history.

- **Port:** `9093` · **DB:** `snop_cockpit` on MariaDB `:30301` (same instance as `microservice_rbac`)
- **Stack:** actix-web 4 · sqlx 0.8 (compile-time `query!` macros) · MariaDB · edition 2024
- **Structure:** mirrors `microservice_rbac` (see “Layout” below)

> ⚠️ Port `9092` is used by the `middlewar` process on this machine — this service deliberately
> uses **9093**.

---

## Why this exists — the three data tiers

The cockpit's Inventory & Purchase screens need three kinds of data:

| Tier | Example | Source | Where |
| --- | --- | --- | --- |
| **1 — Facts** | on-hand, valuation, lead time, open POs, sell-out | ERPNext (read-only) | queried directly by `cockpit-next` |
| **2 — Parameters** | service level, MOQ, increment, order type, EOQ costs, target DIO | *decisions* — not in ERP | **this service** |
| **3 — Computed** | safety stock, ROP, ABC×XYZ, demand stats | derived from 1 + 2 | computed in `cockpit-next`, cached |

Before this service, the frontend **hard-coded a 95% service level** and left MOQ/σLT blank. This
service replaces those assumptions with real, editable, audited values.

> **Scope:** **Part A** (parameters + config + audit) and the **Part B persistence layer** — the
> demand-side write store (planning cycle, rep sales-forecast, DSP consensus picks + freeze). The
> demand *calculation* engine (Slim4 stats, consensus bands, allocation) lives in `cockpit-next`;
> this service only persists and serves what ERP/Slim4 cannot regenerate.

---

## How `cockpit-next` connects to it

The frontend reads this service **server-side**. Integration points (`cockpit-next/src/app/inventory-purchase/`):

- **`snop-be.ts`** — the client: `getStockingPolicyCells(scope)` / `putStockingPolicyCells(cells)`
  (the detailed table) and `loadPlanningParams()` (`GET /config`). Fails soft to built-in defaults.
- **`policy-feed.ts`** — two paths:
  - `refreshStockingPolicy()` computes cells from ERP and `PUT /policy/cells` (the refresh writer);
  - `loadStockingPolicyFeed()` `GET /policy/cells` + config, groups per item, and derives SS/ROP
    from the effective params (`sl_ovr ?? default`, etc.). **No ERP scan on read.**
- **`api/policy-params/[item]/[branch]/route.ts`** — proxy for the table's per-branch edits →
  `PUT /policy/params/{item}/{branch}` (sets the override columns).
- **`src/instrumentation.ts`** — polls the durable schedule every 15 minutes; an atomic lease runs
  `refreshStockingPolicy()` only when `job_schedule.next_run_at` is due.
- **Env:** `cockpit-next/.env.local` sets `SNOP_BE_URL=http://localhost:9093` (gitignored).

So: **run this service before the cockpit.** Verified end-to-end: boot refresh materialized 684
cells; feed reads the table in ~0.18s; override on `1002/Jakarta` (sl 0.99) shows per-branch while
`1002/Surabaya` stays at the 0.95 default, and the ERP `dd` is preserved across the next refresh.

---

## Material Request naming series — production release gate

> ⚠️ **TEST MODE:** the cockpit payload builder and the Frappe Material Request API script
> currently force `company_series` to `TEST-`. **Do not release to production while that
> value remains in either location.**

Before production:

1. In `cockpit-next/src/lib/matreq-parity.ts`, restore the branch-specific series:

   | Request type | Jakarta | Semarang | Surabaya |
   | --- | --- | --- | --- |
   | Purchase | `RO.YY./.MM./B.####` | `RO.YY./.MM./H.####` | `RO.YY./.MM./L.####` |
   | Material Transfer | `MR.YY./.MM./B.####` | `MR.YY./.MM./H.####` | `MR.YY./.MM./L.####` |

2. In `scripts/frappe_create_material_request.py`, remove or replace the forced
   `document_data["company_series"] = "TEST-"` assignment so the script accepts and validates
   the production series sent by the cockpit.
3. Confirm all six production naming series exist in ERPNext, then smoke-test one Purchase and one
   Material Transfer request before launch.

### Material Request origin validation

The Frappe script treats the Item master as the source of truth for each row's mandatory `origin`.
Every submitted Item must exist and have `country_of_origin` populated. The script copies that
master value into the Material Request Item and rejects the entire request when any selected Item
is incomplete.

There is no fallback country. When validation fails, update **Country of Origin** on the affected
ERPNext Item master records, then retry the request.

---

## Data model

| Table | Grain | Columns |
| --- | --- | --- |
| **`stocking_policy`** | **item × branch** | **The detailed materialized table — the feed's source of truth.** ERP-refreshed: `item_name/item_group/primary_item_group/uom`, `stocked`, `cls`, `uc`, `oh_qty/oh_value`, `lead_time`, demand `dd/md/aq/sd/obs`, `refreshed_at`. Planner overrides (win over ERP/default, preserved across refresh): `sl_ovr`, `ot_ovr`, `moq_ovr`, `inc_ovr`, `sigma_lt_ovr`, `override_updated_by/at`. |
| `policy_change_log` | one row per edited field | `item_code`, `branch`, `field`, `from_value`, `to_value`, `actor`, `created_at` |
| `config` | key/value | seeded: `target_dio_days`=100, `default_service_level`=0.95 |
| `job_schedule` | one row per background job | Durable cadence, next run, lease, status, last success, retry and error state. `stocking_policy_refresh` defaults to six calendar months. |
| `eoq_param` | item_group (`''` = company default) | `ordering_cost` (S), `holding_pct` (H), `active` |
| `demand_snapshot` | as_of date | legacy blob cache of the sell-out scan — **superseded by `stocking_policy`**; kept for possible matreq/PO reuse. |

**Two column groups, two writers:** the **refresh job** (`PUT /policy/cells`) upserts only the
ERP-refreshed columns and never touches `*_ovr`; a **planner override** (`PUT /policy/params/{item}/{branch}`)
upserts only the `*_ovr` columns (+ a `policy_change_log` row per changed field). Scheduled and
manual refreshes keep overrides intact. The feed derives SS/ROP from the **effective**
value: `override ?? config default` (service level) / `override ?? null` (MOQ, increment, σLT).

Migrations are in `migrations/` (sqlx, timestamped). Numeric planning fields are `DOUBLE` (→ Rust
`f64`) to avoid the extra `rust_decimal` sqlx feature.

---

## API

Standard envelope: `{ "status_code", "message", "data" }`. Errors map 409 (unique) / 400 (FK) / 404.

| Method | Path | Body | Notes |
| --- | --- | --- | --- |
| GET | `/health` | — | `{ database: "up" }` |
| GET | `/policy/cells` | — | **the feed reads this** — all detailed cells; `?scope=all` for every cell (default = stocked only) |
| PUT | `/policy/cells` | `{ cells: [ …ERP columns… ] }` | **the refresh job writes this** — bulk upsert of ERP columns; overrides preserved |
| GET | `/policy/params/{item}/{branch}` | — | one cell (ERP + overrides); 404 if none |
| PUT | `/policy/params/{item}/{branch}` | `{ field, value, actor? }` | set ONE `*_ovr` override column and log it; `field` ∈ `service_level \| order_type \| lead_time \| sigma_lt \| moq \| increment \| stocked`; `value: null` clears that one field |
| DELETE | `/policy/params/{item}/{branch}` | — | clear the overrides (keeps the ERP row) |
| GET | `/policy/changelog` | — | recent edits; `?item=CODE` to filter |
| GET | `/config` | — | all config rows |
| PUT | `/config/{key}` | `{ value, actor? }` | upsert one key |
| GET | `/job-schedules/{key}` | — | read the persisted next run, cadence, lease, status and last result |
| PUT | `/job-schedules/{key}` | `{ enabled?, interval_months?, next_run_at?, actor? }` | reschedule or enable/disable a job |
| POST | `/job-schedules/{key}/claim` | `{ owner }` | atomic due-job lease; only a `claimed: true` caller runs the work |
| POST | `/job-schedules/{key}/complete` | `{ owner }` | finish the owned lease and advance by the configured calendar months |
| POST | `/job-schedules/{key}/fail` | `{ owner, error? }` | record the error and release the lease for a one-hour retry |
| GET | `/eoq` | — | EOQ rows (global + per item-group) |
| PUT | `/eoq` | `{ item_group, ordering_cost?, holding_pct?, active?, actor? }` | upsert by item_group (`""` = global) |
| GET | `/policy/snapshot?asOf=DATE` | — | materialized sell-out snapshot; 404 if none |
| PUT | `/policy/snapshot` | `{ as_of, payload }` | write-through cache from the cockpit's cold start |
| GET | `/cycle/{cycle_id}` | — | one planning cycle; 404 if none |
| POST | `/cycle/ensure` | `{ cycle_id, label?, target_months? }` | idempotent open-cycle create; `target_months` = JSON array string |
| POST | `/cycle/freeze` | `{ cycle_id, frozen_by }` | atomically lock consensus + cycle; 409 if closed, empty, or missing a target month; repeat freeze is a no-op |
| GET | `/sales-forecast?cycle_id=…&salesperson=…` | — | rep Sales-Forecast entries for a cycle (optional rep filter) |
| POST | `/sales-forecast` | `{ cycle_id, updated_by, entries[] }` | upsert rep forecasts; 409 if cycle not open |
| GET | `/demand-consensus?cycle_id=…` | — | DSP consensus picks for a cycle |
| POST | `/demand-consensus` | `{ cycle_id, resolved_by, picks[] }` | upsert picks (skips frozen rows); 409 if cycle not open |

Examples:

```bash
# Set a per item×branch override — ONE field per call, attributed to a user
curl -X PUT localhost:9093/policy/params/1196/Jakarta \
  -H 'content-type: application/json' \
  -d '{"field":"service_level","value":0.98,"actor":"alfin@bahteraadijaya.com"}'
# Clear a single override back to the ERP-derived value
curl -X PUT localhost:9093/policy/params/1196/Jakarta \
  -H 'content-type: application/json' \
  -d '{"field":"moq","value":null,"actor":"alfin@bahteraadijaya.com"}'

# Raise the company default service level
curl -X PUT localhost:9093/config/default_service_level \
  -H 'content-type: application/json' -d '{"value":"0.97","actor":"pm"}'

# Activate EOQ once Finance provides S and H
curl -X PUT localhost:9093/eoq \
  -H 'content-type: application/json' \
  -d '{"item_group":"","ordering_cost":250000,"holding_pct":0.22,"active":true}'
```

---

## Run

```bash
cp .env .env.local     # optional; edit DB creds there. .env already points at snop_cockpit :30301
make setup             # sqlx migrate + seed defaults + example overrides
make run               # http://localhost:9093   (make dev = auto-reload; make test = native Rust)
```

`make test` runs `cargo run --bin test`. The native Rust executable creates a uniquely named
temporary MariaDB schema, applies migrations and seed data, exercises every endpoint in-process
(including scheduling and the change-log audit), then drops only that temporary schema. It never
resets or mutates the configured application database. You can invoke it directly:

```bash
cargo run --bin test
```

---

## Layout (same as `microservice_rbac`)

```
src/
  main.rs                  # actix bootstrap, tracing, binds :9093
  api/mod.rs               # ApiResponse envelopes, db_error/affected/not_found, /health, route wiring
  api/{policy,config,eoq}.rs   # thin handlers → controllers
  module/<x>/model.rs      # row structs (FromRow) + request bodies
  module/<x>/controller.rs # the sqlx query! logic
  utils/db.rs              # custom deadpool Pool that impls sqlx::Executor (so `&state.db_pool` works)
  bin/seed.rs              # applies scripts/seed.sql
migrations/                # sqlx migrations (timestamped)
```

Conventions: compile-time `sqlx::query!` / `query_as!` (needs `DATABASE_URL` reachable at build —
`make migrate` first), bool columns need an `AS "x: bool"` hint, `cargo fmt` (8-space, no hard tabs).

---

## Materialization & auto-update (done)

The feed **reads `stocking_policy` directly** — the ~24s trailing-12 ERP scan happens only in the
refresh job, never on a user read. `cockpit-next/src/instrumentation.ts` checks the own-DB schedule
on boot and every 15 minutes. A refresh runs only after the atomic lease reports `claimed: true`;
Stocking Policy now uses a 24-hour operational cadence, while `interval_months` remains available
for slower reconciliation jobs. Failure records the error and retries after one hour. Polling is
aligned to real quarter-hour boundaries. The persisted row survives restarts and prevents duplicate
runs across workers.
`PUT /policy/cells` preserves all planner overrides, and `?refresh=1` remains available for a manual
inline recompute. Set `SNAPSHOT_WARM=off` to disable the in-app scheduler.

> `stocking_policy` currently combines slow policy classification with operational `oh_qty`,
> `oh_value`, valuation and demand fields. Until those grains are split into separate jobs, the
> combined materialization refreshes daily and page reads keep serving the last successful snapshot.

> The old `demand_snapshot` blob + `/policy/snapshot` endpoints still exist but are legacy — the
> detailed `stocking_policy` table is now the source of truth.

## Not built yet (own-DB follow-ons, still Part A)

- **Material Request proposals** + **Purchase Overview action persistence** (expedite/postpone
  approvals) — the write-back/audit side of those two screens.
- RBAC/persona is handled by `microservice_rbac`, not here.
- 
