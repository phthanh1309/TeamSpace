//! Password hashing.
//!
//! TeamSpace stores only Argon2id PHC strings. The hash embeds the algorithm,
//! parameters and a per-password random salt, so no separate salt column is
//! needed. Plaintext passwords never leave this module.

use argon2::Argon2;
use argon2::password_hash::{self, PasswordHasher, PasswordVerifier, phc::PasswordHash};

/// Shortest password accepted at registration.
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Longest password accepted at registration.
///
/// This is a denial-of-service guard, not a truncation limit: longer input is
/// rejected rather than silently shortened.
pub const MAX_PASSWORD_LENGTH: usize = 1024;

/// Hashes a password with Argon2id and returns its PHC string.
///
/// A fresh cryptographically secure salt is generated for every call, so
/// hashing the same password twice yields different encodings.
pub fn hash_password(password: &str) -> Result<String, password_hash::Error> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
}

/// Returns whether `password` matches the encoded Argon2 password hash.
///
/// A malformed or unsupported hash is treated as a non-match.
#[must_use]
pub fn verify_password(password: &str, encoded_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(encoded_hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    const PASSWORD: &str = "correct horse battery staple";

    #[test]
    fn hash_verifies_against_the_original_password() {
        let hash = hash_password(PASSWORD).expect("hash a valid password");

        assert!(verify_password(PASSWORD, &hash));
    }

    #[test]
    fn hash_is_not_the_plaintext_password() {
        let hash = hash_password(PASSWORD).expect("hash a valid password");

        assert_ne!(hash, PASSWORD);
        assert!(!hash.contains(PASSWORD));
    }

    #[test]
    fn repeated_hashing_uses_a_fresh_salt() {
        let first = hash_password(PASSWORD).expect("hash a valid password");
        let second = hash_password(PASSWORD).expect("hash a valid password");

        assert_ne!(first, second, "each hash must embed a distinct salt");
        assert!(verify_password(PASSWORD, &first));
        assert!(verify_password(PASSWORD, &second));
    }

    #[test]
    fn wrong_password_does_not_verify() {
        let hash = hash_password(PASSWORD).expect("hash a valid password");

        assert!(!verify_password("wrong password", &hash));
    }

    #[test]
    fn malformed_hash_does_not_verify() {
        assert!(!verify_password(PASSWORD, "not-a-password-hash"));
    }

    #[test]
    fn hash_is_an_argon2id_phc_string() {
        let hash = hash_password(PASSWORD).expect("hash a valid password");

        assert!(
            hash.starts_with("$argon2id$v=19$"),
            "unexpected hash format: {hash}"
        );
    }
}
