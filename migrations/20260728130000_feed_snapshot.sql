-- Generic materialized-feed cache, keyed by an arbitrary snapshot key (not a date like
-- demand_snapshot). Lets expensive derived feeds (e.g. principal performance: sell-in YTD +
-- run-rate + GM% by principal) be computed once by the scheduled refresh and read back cheaply,
-- instead of re-scanning ERP on every page load. payload = opaque JSON owned by the frontend.
CREATE TABLE feed_snapshot (
    snapshot_key VARCHAR(64) NOT NULL,
    payload LONGTEXT NOT NULL,
    computed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (snapshot_key)
) ENGINE=InnoDB;
