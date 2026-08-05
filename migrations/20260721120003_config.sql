-- Company-wide planning config as key/value. Seeded defaults: target_dio_days=100,
-- default_service_level=0.95 (the value the Stocking Policy screen currently assumes).
CREATE TABLE config (
    config_key VARCHAR(64) NOT NULL,
    config_value VARCHAR(255) NOT NULL,
    updated_by VARCHAR(255) NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (config_key)
) ENGINE=InnoDB;
