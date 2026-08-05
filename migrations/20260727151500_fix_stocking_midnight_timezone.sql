-- sqlx connections use UTC while interactive MariaDB sessions use +07:00. Convert through the
-- active session timezone so the stored TIMESTAMP always represents 00:00 Asia/Jakarta.
UPDATE job_schedule
SET next_run_at = CONVERT_TZ(
        DATE_ADD(
            DATE(
                CONVERT_TZ(
                    COALESCE(last_success_at, CURRENT_TIMESTAMP(6)),
                    @@session.time_zone,
                    '+07:00'
                )
            ),
            INTERVAL interval_months MONTH
        ),
        '+07:00',
        @@session.time_zone
    ),
    updated_by = 'migration:midnight-wib-fixed'
WHERE job_key = 'stocking_policy_refresh';
