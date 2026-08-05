-- Rep Sales-Forecast input — ONE record per cycle × customer × item × branch × month.
-- Salesperson is a column (the customer's owner / last writer), NOT part of the key, so a
-- re-submit OVERRIDES the previous value. sales_qty NULL = accept the baseline (DSC) for
-- that month. Mirrors the frontend write store.
CREATE TABLE IF NOT EXISTS sales_forecast_entry (
    cycle_id      VARCHAR(32)  NOT NULL,
    salesperson   VARCHAR(128) NOT NULL,
    customer      VARCHAR(255) NOT NULL,
    item_code     VARCHAR(140) NOT NULL,
    branch        VARCHAR(64)  NOT NULL,
    ym            VARCHAR(7)   NOT NULL,                -- '2026-08'
    sales_qty     DOUBLE       NULL,                   -- null = accept baseline (DOUBLE → Rust f64)
    sales_reason  ENUM('New Project','Lost','Won Tender','Promotion') NULL,
    selected      ENUM('Consensus','Sales','Historical','Manual') NULL,
    updated_by    VARCHAR(128) NOT NULL,
    updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (cycle_id, customer, item_code, branch, ym),
    KEY sfe_join_idx (cycle_id, item_code, branch, ym)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
