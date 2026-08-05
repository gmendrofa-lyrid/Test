-- Ownership snapshot: which salesperson invoiced each customer × item × branch. Seeded from ERP
-- Sales Invoices by the cockpit (READ side stays in cockpit-next); this service persists/serves it.
-- Customer-grain keys mirror snop/migrations/002_customer_grain.sql.
CREATE TABLE IF NOT EXISTS rep_assignment (
    cycle_id     VARCHAR(32)  NOT NULL,
    salesperson  VARCHAR(128) NOT NULL,
    item_code    VARCHAR(140) NOT NULL,
    branch       VARCHAR(64)  NOT NULL,                 -- Jakarta/Surabaya/Semarang
    customer     VARCHAR(255) NOT NULL DEFAULT '',
    bu           VARCHAR(64)  NULL,
    source       ENUM('erp_invoice','manual') NOT NULL DEFAULT 'erp_invoice',
    built_at     DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (cycle_id, salesperson, customer, item_code, branch),
    KEY rep_assignment_join_idx (cycle_id, item_code, branch),
    KEY rep_assignment_customer_idx (cycle_id, customer, item_code, branch)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
