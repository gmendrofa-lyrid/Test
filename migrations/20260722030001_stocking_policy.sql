-- Detailed materialized Stocking Policy, one row per item × branch. Supersedes the blob
-- `demand_snapshot` as the feed's source of truth. Two column groups:
--   • ERP-refreshed  — overwritten by the periodic refresh job (frontend computes from ERP);
--   • *_ovr override — set by planners, NEVER touched by the refresh; win over the ERP/default.
-- The feed SELECTs this table and derives SS/ROP with the effective (override ?? default) params.
CREATE TABLE stocking_policy (
    item_code VARCHAR(64) NOT NULL,
    branch VARCHAR(64) NOT NULL,

    -- ERP-refreshed (identity + facts + demand statistics)
    item_name VARCHAR(255) NULL,
    item_group VARCHAR(255) NULL,
    primary_item_group VARCHAR(255) NULL,
    uom VARCHAR(32) NULL,
    stocked BOOLEAN NOT NULL DEFAULT FALSE,
    cls VARCHAR(8) NULL,                 -- ABC×XYZ, e.g. "A·X"
    uc DOUBLE NULL,                       -- weighted valuation
    oh_qty DOUBLE NOT NULL DEFAULT 0,     -- on-hand qty (as-of last refresh)
    oh_value DOUBLE NOT NULL DEFAULT 0,   -- on-hand value (IDR)
    lead_time DOUBLE NULL,                -- days (per-supplier, from ERP)
    dd DOUBLE NOT NULL DEFAULT 0,         -- mean daily demand
    md DOUBLE NOT NULL DEFAULT 0,         -- mean monthly demand
    aq DOUBLE NOT NULL DEFAULT 0,         -- annual demand
    sd DOUBLE NOT NULL DEFAULT 0,         -- σ daily demand
    obs INT NOT NULL DEFAULT 0,           -- months with sell-out (of 12)
    refreshed_at TIMESTAMP NULL,

    -- Override (planner-set; take precedence; preserved across refreshes)
    sl_ovr DOUBLE NULL,                   -- service level
    ot_ovr VARCHAR(32) NULL,              -- order type
    moq_ovr DOUBLE NULL,
    inc_ovr DOUBLE NULL,
    sigma_lt_ovr DOUBLE NULL,
    override_updated_by VARCHAR(255) NULL,
    override_updated_at TIMESTAMP NULL,

    PRIMARY KEY (item_code, branch),
    KEY idx_sp_stocked (stocked),
    KEY idx_sp_item (item_code)
) ENGINE=InnoDB;

-- Overrides now live in stocking_policy.*_ovr; the standalone params table is retired.
-- (policy_change_log stays — it's the edit audit trail.)
DROP TABLE IF EXISTS policy_param;
