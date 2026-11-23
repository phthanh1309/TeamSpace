-- Roll back to the original case-sensitive UNIQUE(email) constraint.
--
-- The original casing of pre-existing addresses cannot be restored; rows stay
-- lowercased, which is semantically equivalent for TeamSpace.
DROP INDEX IF EXISTS users_email_lower_key;

ALTER TABLE users
    ADD CONSTRAINT users_email_key UNIQUE (email);
