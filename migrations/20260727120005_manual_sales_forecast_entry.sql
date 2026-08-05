-- Manual rep Sales-Forecast input as captured in the master_fc_sales.xlsx FORECAST sheet
-- (the number each salesperson typed by hand, per customer × item × branch × month). Same shape
-- as sales_forecast_entry so the two can be diffed/overlaid; this table is the imported manual
-- snapshot rather than the app-write store. sales_qty NULL = the sales left the cell blank ('-').
CREATE TABLE IF NOT EXISTS manual_sales_forecast_entry (
    cycle_id      VARCHAR(32)  NOT NULL,
    salesperson   VARCHAR(128) NOT NULL,
    customer      VARCHAR(255) NOT NULL,
    item_code     VARCHAR(140) NOT NULL,
    branch        VARCHAR(64)  NOT NULL,
    ym            VARCHAR(7)   NOT NULL,                -- '2026-08'
    sales_qty     DOUBLE       NULL,                   -- null = blank/'-' in the sheet
    sales_reason  ENUM('New Project','Lost','Won Tender','Promotion') NULL,
    selected      ENUM('Consensus','Sales','Historical','Manual') NULL,
    updated_by    VARCHAR(128) NOT NULL,
    updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (cycle_id, salesperson, customer, item_code, branch, ym),
    KEY msfe_join_idx (cycle_id, item_code, branch, ym),
    KEY msfe_customer_idx (cycle_id, customer, item_code, branch, ym)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
