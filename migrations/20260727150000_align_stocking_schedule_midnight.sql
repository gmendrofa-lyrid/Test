-- The S&OP database runs in Asia/Jakarta (+07:00). Align the already-seeded six-month Stocking
-- Policy run to local midnight; future successful runs use the same rule in the controller.
UPDATE job_schedule
SET next_run_at = DATE(next_run_at),
    updated_by = 'migration:midnight-wib'
WHERE job_key = 'stocking_policy_refresh';
