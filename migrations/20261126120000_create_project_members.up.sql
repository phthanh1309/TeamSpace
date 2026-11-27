-- Additional collaborators on a project.
--
-- Ownership is *not* stored here: `projects.owner_id` remains the single source
-- of truth for the owner, and the owner must never be inserted as a member.
-- This table only holds the extra `member` role.
--
-- The composite primary key makes a duplicate membership impossible and already
-- serves lookups by project, which is the access check used by every read.
--
-- Deleting a project cascades: a membership has no meaning without its project.
-- Deleting a user also cascades: a membership is not content. A user who owns
-- work is still protected by the RESTRICT foreign key on `projects.owner_id`.
CREATE TABLE project_members (
    project_id BIGINT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, user_id)
);

-- Serves "projects this user belongs to" for the project list and the foreign
-- key check performed when a user is deleted. The primary key already covers
-- the project side.
CREATE INDEX project_members_user_id_idx ON project_members (user_id);
