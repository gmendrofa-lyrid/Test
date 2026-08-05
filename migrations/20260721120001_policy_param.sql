-- Tier-2 stocking-policy parameter OVERRIDES, per item × branch.
-- A row exists only when a planner has set at least one value; absence means "use the default"
-- (default service level lives in `config`). NULL columns fall back to defaults individually.
CREATE TABLE policy_param (
    item_code VARCHAR(64) NOT NULL,
    branch VARCHAR(64) NOT NULL,
    service_level DOUBLE NULL,        -- fraction, e.g. 0.95
    order_type VARCHAR(32) NULL,      -- planned | buy | transfer | work_order
    moq DOUBLE NULL,                  -- minimum order qty (stock UoM)
    increment DOUBLE NULL,            -- order/pack increment
    sigma_lt DOUBLE NULL,             -- lead-time std dev (days)
    updated_by VARCHAR(255) NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (item_code, branch)
) ENGINE=InnoDB;
