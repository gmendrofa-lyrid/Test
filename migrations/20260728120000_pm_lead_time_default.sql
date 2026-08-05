-- PM-maintained lead-time defaults, used while the real PO->GR export is missing (BUSINESS_RULES
-- 5.5). Grain: principal x PIG x item_group x leg. Legs (config.py:140):
--   supplier_dispatch, ocean_air_transit, customs, bpom_permit, qc_release.
-- Any of principal/pig/item_group may be '' to mean "applies to all" at that axis (most-specific
-- match wins in the engine). Planning percentile is applied downstream; here we store mean + sigma.
CREATE TABLE pm_lead_time_default (
    id CHAR(36) NOT NULL,
    principal VARCHAR(128) NOT NULL DEFAULT '',   -- '' = any principal
    pig VARCHAR(128) NOT NULL DEFAULT '',          -- '' = any PIG
    item_group VARCHAR(128) NOT NULL DEFAULT '',   -- '' = any item group
    leg VARCHAR(32) NOT NULL,                       -- one of the five lead-time legs
    lead_days DOUBLE NOT NULL DEFAULT 0,            -- PM default mean lead days for this leg
    sigma_days DOUBLE NOT NULL DEFAULT 0,          -- PM default sigma (day-to-day variability)
    note VARCHAR(255) NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_by VARCHAR(255) NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_pm_lt (principal, pig, item_group, leg)
) ENGINE=InnoDB;
