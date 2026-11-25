-- Projects owned by a single user for the first milestone.
--
-- Ownership lives in exactly one place: `projects.owner_id`. Later milestones
-- add `project_members` for collaborators, but that table must not store a
-- duplicate "owner" role; ownership is always read from this column so the two
-- can never disagree.
--
-- The foreign key uses ON DELETE RESTRICT. Projects will be shared with other
-- members, so deleting a user must not silently delete a project and all of its
-- work; account deletion has to transfer ownership or delete projects first.
-- (`sessions` cascades because a session has no value without its user.)
CREATE TABLE projects (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name        VARCHAR(120) NOT NULL,
    description TEXT,
    owner_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT projects_name_not_blank CHECK (btrim(name) <> ''),
    CONSTRAINT projects_description_length CHECK (
        description IS NULL OR char_length(description) <= 2000
    )
);

-- Serves the ownership filter used by every query together with the list
-- ordering (`ORDER BY created_at DESC, id DESC`). The leading column also
-- supports the foreign-key check performed when a user is deleted.
CREATE INDEX projects_owner_created_at_idx
    ON projects (owner_id, created_at DESC, id DESC);
