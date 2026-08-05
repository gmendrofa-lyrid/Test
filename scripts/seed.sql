-- Default planning config + a company-wide (inactive) EOQ row. Idempotent.
INSERT INTO config (config_key, config_value) VALUES
    ('target_dio_days', '100'),
    ('default_service_level', '0.95')
ON DUPLICATE KEY UPDATE config_value = VALUES(config_value);

INSERT INTO eoq_param (id, item_group, ordering_cost, holding_pct, active)
VALUES ('00000000-0000-0000-0000-000000000000', '', NULL, NULL, FALSE)
ON DUPLICATE KEY UPDATE item_group = item_group;

-- Open the current demand-planning cycle (DV_TARGET window 2026-08..2027-01) so the demand-side
-- endpoints (cycle / sales-forecast / demand-consensus) have an open cycle out of the box.
INSERT INTO sop_cycle (cycle_id, label, target_months, status)
VALUES ('2026-08', '2026-08',
        '["2026-08","2026-09","2026-10","2026-11","2026-12","2027-01"]', 'open')
ON DUPLICATE KEY UPDATE cycle_id = cycle_id;
