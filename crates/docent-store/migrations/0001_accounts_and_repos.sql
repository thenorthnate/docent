-- Connected forge accounts. One login per host, and never a token: tokens
-- are read from `gh` and held in memory only.
CREATE TABLE accounts (
    id           INTEGER PRIMARY KEY,
    forge        TEXT NOT NULL,                -- github
    host         TEXT NOT NULL COLLATE NOCASE,
    login        TEXT NOT NULL,
    auth_source  TEXT NOT NULL,                -- gh
    keychain_ref TEXT,
    UNIQUE (forge, host)
);

-- Forge repos Docent knows about. A repo stays after it is unwatched, so its
-- binding to a local clone survives watching it again.
CREATE TABLE repos (
    id             INTEGER PRIMARY KEY,
    account_id     INTEGER NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    -- Forge repo names are case-insensitive.
    owner          TEXT NOT NULL COLLATE NOCASE,
    name           TEXT NOT NULL COLLATE NOCASE,
    default_branch TEXT NOT NULL,
    local_path     TEXT,
    watched        INTEGER NOT NULL DEFAULT 1 CHECK (watched IN (0, 1)),
    profile_id     INTEGER,
    UNIQUE (account_id, owner, name)
);
