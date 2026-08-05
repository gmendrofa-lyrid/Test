-- Durable scheduler state for expensive materialization jobs.
--
-- `next_run_at` is persisted so application restarts do not reset the cadence. A short lease makes
-- the claim safe when multiple Next.js processes poll at the same time.
CREATE TABLE job_schedule (
    job_key VARCHAR(64) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    interval_months SMALLINT UNSIGNED NOT NULL DEFAULT 6,
    next_run_at TIMESTAMP(6) NOT NULL,
    retry_after_at TIMESTAMP(6) NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'idle',
    last_run_started_at TIMESTAMP(6) NULL,
    last_run_finished_at TIMESTAMP(6) NULL,
    last_success_at TIMESTAMP(6) NULL,
    lease_owner VARCHAR(128) NULL,
    lease_expires_at TIMESTAMP(6) NULL,
    last_error TEXT NULL,
    row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
    updated_by VARCHAR(255) NULL,
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (job_key),
    INDEX idx_job_schedule_due (enabled, next_run_at),
    CONSTRAINT chk_job_schedule_interval CHECK (interval_months BETWEEN 1 AND 120),
    CONSTRAINT chk_job_schedule_status
        CHECK (status IN ('idle', 'running', 'failed', 'disabled'))
) ENGINE=InnoDB;

-- Preserve the latest known successful materialization as the cadence anchor. For the current
-- dataset this schedules the next Stocking Policy run exactly six calendar months after the latest
-- `refreshed_at`; an empty table instead schedules six months from migration time.
INSERT INTO job_schedule (
    job_key,
    enabled,
    interval_months,
    next_run_at,
    status,
    last_run_started_at,
    last_run_finished_at,
    last_success_at,
    updated_by
)
SELECT
    'stocking_policy_refresh',
    TRUE,
    6,
    DATE_ADD(COALESCE(MAX(refreshed_at), UTC_TIMESTAMP()), INTERVAL 6 MONTH),
    'idle',
    MIN(refreshed_at),
    MAX(refreshed_at),
    MAX(refreshed_at),
    'migration'
FROM stocking_policy;
