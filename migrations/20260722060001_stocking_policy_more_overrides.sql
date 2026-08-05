-- Every editable column in the Stocking Policy table needs an override slot, so a planner edit to
-- ANY field persists. Adds the two that were missing: stocked flag and lead time.
ALTER TABLE stocking_policy
    ADD COLUMN st_ovr BOOLEAN NULL AFTER sigma_lt_ovr,
    ADD COLUMN lt_ovr DOUBLE NULL AFTER st_ovr;
