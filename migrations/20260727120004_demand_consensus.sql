-- DSP consensus picks per cycle × customer × item × branch × month. When is_frozen=1 the
-- selected_qty is plan-of-record and must not be overwritten by later edits (enforced in the
-- controller). `reason` is required when selected_source='Manual' (enforced in the app).
CREATE TABLE IF NOT EXISTS demand_consensus (
    cycle_id         VARCHAR(32)  NOT NULL,
    customer         VARCHAR(255) NOT NULL,
    item_code        VARCHAR(140) NOT NULL,
    branch           VARCHAR(64)  NOT NULL,
    ym               VARCHAR(7)   NOT NULL,
    selected_source  ENUM('Consensus','Sales','Historical','Manual') NOT NULL,
    selected_qty     DOUBLE       NOT NULL,             -- DOUBLE → Rust f64
    reason           VARCHAR(255) NULL,                 -- required when Manual (enforced in app)
    auto_accepted    TINYINT(1)   NOT NULL DEFAULT 0,
    resolved_by      VARCHAR(128) NULL,
    resolved_at      DATETIME     NULL,
    frozen_by        VARCHAR(128) NULL,
    frozen_at        DATETIME     NULL,
    is_frozen        TINYINT(1)   NOT NULL DEFAULT 0,
    PRIMARY KEY (cycle_id, customer, item_code, branch, ym),
    KEY dc_join_idx (cycle_id, item_code, branch, ym),
    KEY dc_frozen_idx (cycle_id, is_frozen)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
