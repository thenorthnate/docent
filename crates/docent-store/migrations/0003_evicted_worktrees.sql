-- An evicted worktree's folder is gone, but its PR's `refs/docent/*` pins
-- stay in the clone until the PR closes or merges. Its row stays too, marked
-- here, so those pins can be found and removed then.
ALTER TABLE worktrees
    ADD COLUMN evicted INTEGER NOT NULL DEFAULT 0 CHECK (evicted IN (0, 1));
