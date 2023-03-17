-- SQLite has horrible ALTER TABLE support, so to revert, we must make a new table and rename.

CREATE TABLE __migration_2023_03_17_191246_add_default_prefix__down__temporary_guilds (
    id TEXT NOT NULL PRIMARY KEY,
    prefix TEXT
);

INSERT INTO __migration_2023_03_17_191246_add_default_prefix__down__temporary_guilds
SELECT * FROM guilds;

DROP TABLE guilds;

ALTER TABLE __migration_2023_03_17_191246_add_default_prefix__down__temporary_guilds
RENAME TO guilds;