-- Stocking Policy classification is ABC value × XYZ demand variability only.
-- Remove the retired gross-margin tier from valid three-part classes while leaving NULL or
-- malformed values untouched so the application can continue quarantining them explicitly.
UPDATE stocking_policy
SET cls = CONCAT(
    SUBSTRING_INDEX(cls, '·', 1),
    '·',
    SUBSTRING_INDEX(cls, '·', -1)
)
WHERE BINARY cls REGEXP '^[ABC]·[HML]·[XYZ]$';
