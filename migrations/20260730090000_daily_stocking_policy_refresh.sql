-- Keep Stocking Policy reads materialized and fast, but refresh the combined operational snapshot
-- daily instead of letting on-hand, valuation, demand, and lead-time facts age for six months.
--
-- `interval_months` remains the backwards-compatible cadence for slower jobs. A non-NULL
-- `interval_hours` takes precedence when a job completes successfully.
ALTER TABLE job_schedule
    ADD COLUMN interval_hours SMALLINT UNSIGNED NULL AFTER interval_months,
    ADD CONSTRAINT chk_job_schedule_interval_hours
        CHECK (interval_hours IS NULL OR interval_hours BETWEEN 1 AND 8760);

-- Schedule the first daily run for the next 00:00 Asia/Jakarta. The application polls every
-- 15 minutes and keeps serving the last successful materialization while a refresh is running.
UPDATE job_schedule
SET interval_hours = 24,
    next_run_at = CASE
        WHEN status = 'running' THEN next_run_at
        ELSE CONVERT_TZ(
            DATE_ADD(
                DATE(CONVERT_TZ(UTC_TIMESTAMP(6), '+00:00', '+07:00')),
                INTERVAL 1 DAY
            ),
            '+07:00',
            @@session.time_zone
        )
    END,
    updated_by = 'migration:daily-stocking-policy',
    row_version = row_version + 1
WHERE job_key = 'stocking_policy_refresh';
