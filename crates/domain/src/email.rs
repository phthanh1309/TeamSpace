//! Email handling shared by the backend.

/// Returns the canonical form of an email address for storage and lookup.
///
/// TeamSpace compares email addresses case-insensitively. The canonical form is
/// therefore the address with surrounding whitespace removed and every character
/// lowercased. This matches the database's unique index on `lower(email)`.
///
/// The function is idempotent: normalizing an already normalized address returns
/// it unchanged.
#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize_email;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            normalize_email(" \t Alice@example.com \r\n "),
            "alice@example.com"
        );
    }

    #[test]
    fn lowercases_the_whole_address() {
        assert_eq!(normalize_email("Alice@Example.COM"), "alice@example.com");
    }

    #[test]
    fn leaves_an_already_normalized_address_unchanged() {
        assert_eq!(normalize_email("alice@example.com"), "alice@example.com");
    }

    #[test]
    fn casing_and_whitespace_variants_share_one_canonical_form() {
        let variants = [
            "Alice@example.com",
            "alice@example.com",
            "ALICE@EXAMPLE.COM",
            "  AlIcE@Example.com  ",
        ];

        for variant in variants {
            assert_eq!(normalize_email(variant), "alice@example.com");
        }
    }

    #[test]
    fn normalization_is_idempotent() {
        let once = normalize_email("  Bob@Example.COM ");
        assert_eq!(normalize_email(&once), once);
    }
}
