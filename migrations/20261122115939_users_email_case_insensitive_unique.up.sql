-- Make users.email case-insensitively unique.
--
-- The initial migration declared `email VARCHAR(255) NOT NULL UNIQUE`, which is
-- case-sensitive: Alice@example.com, alice@example.com and ALICE@EXAMPLE.COM
-- could all coexist. TeamSpace treats email addresses as case-insensitive, so
-- the old constraint is replaced with a unique expression index on lower(email).
-- Keeping both mechanisms would enforce two different rules on the same column.

-- Abort before modifying any row if existing data contains addresses that only
-- differ by casing (or surrounding whitespace). Nothing is deleted or merged
-- automatically: an operator must resolve the conflict first.
DO $$
DECLARE
    duplicate_groups BIGINT;
BEGIN
    SELECT count(*)
    INTO duplicate_groups
    FROM (
        SELECT lower(btrim(email))
        FROM users
        GROUP BY lower(btrim(email))
        HAVING count(*) > 1
    ) duplicates;

    IF duplicate_groups > 0 THEN
        RAISE EXCEPTION
            'users.email contains % group(s) of addresses that collide after trimming and lowercasing. Resolve them manually, then re-run. Inspect with: SELECT lower(btrim(email)), count(*) FROM users GROUP BY 1 HAVING count(*) > 1;',
            duplicate_groups;
    END IF;
END
$$;

-- Store the canonical form the application uses from now on.
UPDATE users
SET email = lower(btrim(email))
WHERE email <> lower(btrim(email));

-- Drop the case-sensitive UNIQUE(email) constraint (and its index); the
-- expression index below replaces it.
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_email_key;

CREATE UNIQUE INDEX users_email_lower_key ON users (lower(email));
