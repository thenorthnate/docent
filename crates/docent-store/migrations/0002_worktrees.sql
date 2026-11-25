-- PR worktrees Docent has created. Each one's path follows from its repo id
-- and PR number, so it isn't stored.
CREATE TABLE worktrees (
    repo_id     INTEGER NOT NULL REFERENCES repos (id),
    number      INTEGER NOT NULL,
    -- The clone and remote it came from. The repo may since have been unbound
    -- or bound to another clone, and removing the worktree needs this one.
    clone_path  TEXT NOT NULL,
    remote      TEXT NOT NULL,
    -- Milliseconds since 1970; 0 until an open works. Later than every other
    -- row's when set, so it orders worktrees by use.
    last_opened INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (repo_id, number)
);

-- The user's choices. A setting with no row has its default.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
