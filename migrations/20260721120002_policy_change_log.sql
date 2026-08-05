-- Audit trail for stocking-policy parameter edits. One row per changed field, so a proposal /
-- review-before-apply flow and "who changed what" are both reconstructable. Never deleted.
CREATE TABLE policy_change_log (
    id CHAR(36) NOT NULL,
    item_code VARCHAR(64) NOT NULL,
    branch VARCHAR(64) NOT NULL,
    field VARCHAR(32) NOT NULL,       -- service_level | order_type | moq | increment | sigma_lt
    from_value VARCHAR(255) NULL,
    to_value VARCHAR(255) NULL,
    actor VARCHAR(255) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_changelog_item (item_code, branch),
    KEY idx_changelog_created (created_at)
) ENGINE=InnoDB;
