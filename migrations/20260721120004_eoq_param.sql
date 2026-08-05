-- EOQ cost parameters (from Finance): ordering cost S + annual holding rate H.
-- item_group = '' is the company-wide default; a named item_group overrides it.
-- EOQ stays inactive until active=TRUE and both costs are set.
CREATE TABLE eoq_param (
    id CHAR(36) NOT NULL,
    item_group VARCHAR(128) NOT NULL DEFAULT '',  -- '' = company-wide default
    ordering_cost DOUBLE NULL,                     -- S: cost per PO (IDR)
    holding_pct DOUBLE NULL,                        -- H: annual holding rate (fraction of unit cost)
    active BOOLEAN NOT NULL DEFAULT FALSE,
    updated_by VARCHAR(255) NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_eoq_group (item_group)
) ENGINE=InnoDB;
