-- S&OP planning cycle status (demand-side / "Part B"). One row per cycle; status governs writes:
--   open   → editable; consensus provisional
--   frozen → demand_consensus.selected_qty is locked plan-of-record; edits rejected
--   closed → archived, read-only
-- Mirrors the frontend write store (snop/migrations/001_snop.sql) so both the cockpit-next
-- direct-SQL path and this service operate on the same snop_cockpit tables.
CREATE TABLE IF NOT EXISTS sop_cycle (
    cycle_id       VARCHAR(32)  NOT NULL,               -- e.g. '2026-08'
    label          VARCHAR(128) NOT NULL,
    target_months  JSON         NOT NULL,               -- DV_TARGET window (JSON array)
    status         ENUM('open','frozen','closed') NOT NULL DEFAULT 'open',
    opened_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    frozen_at      DATETIME     NULL,
    closed_at      DATETIME     NULL,
    PRIMARY KEY (cycle_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
