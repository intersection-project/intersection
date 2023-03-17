-- SQLite has horrible ALTER TABLE support, so to add the default prefix of '+'
-- for the new table, we must create a new table and copy data over.

CREATE TABLE __migration_2023_03_17_191246_add_default_prefix__up__temporary_guilds (
    id TEXT NOT NULL PRIMARY KEY,
    prefix TEXT NOT NULL DEFAULT '+'
);

INSERT INTO __migration_2023_03_17_191246_add_default_prefix__up__temporary_guilds
SELECT * FROM guilds;

DROP TABLE guilds;

ALTER TABLE __migration_2023_03_17_191246_add_default_prefix__up__temporary_guilds
RENAME TO guilds;