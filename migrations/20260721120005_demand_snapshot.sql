-- Materialized sell-out snapshot for the Stocking Policy feed. The cockpit's expensive trailing-12
-- ERP scan (stocked set + monthly demand per item×branch, ~24s) is written here once per as-of date
-- so cold starts read it back instead of re-scanning ERP. payload = opaque JSON owned by the
-- frontend ({ stocked: [...], series: { "item|branch": [12] } }); one row per as-of.
CREATE TABLE demand_snapshot (
    as_of DATE NOT NULL,
    payload LONGTEXT NOT NULL,
    computed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (as_of)
) ENGINE=InnoDB;
