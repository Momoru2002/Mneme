//! App-lock: a single local password that gates the desktop app's commands.
//!
//! This is NOT the multi-user auth/RBAC system that `rbac.rs`'s doc comment
//! says was removed — it's new, deliberately smaller: one password for the
//! one local owner, checked once per app launch, held only in memory for the
//! life of the process (never written to disk, never sent anywhere). Its only
//! job is to stop someone who picks up an unlocked laptop, or who obtains a
//! Web Mode session token, from reading or changing notes without also
//! knowing this password.
//!
//! Hashing uses argon2id (the `argon2` dependency already in Cargo.toml —
//! previously unused dead weight left over from before the old auth system
//! was stripped out; see `rbac.rs` and `services/owner.rs`).

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use password_hash::rand_core::OsRng;
use std::sync::Mutex;

/// Hash `password` with argon2id and a freshly-generated random salt.
/// Returns the standard PHC string format (`$argon2id$v=19$...`), which
/// self-describes its own salt and parameters — nothing else needs storing
/// alongside it to verify later.
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("failed to hash password: {e}"))
}

/// Verify `password` against a stored PHC hash string from [`hash_password`].
/// Returns `Ok(false)` (not an error) for a simple wrong-password mismatch;
/// `Err` only for a malformed/corrupt stored hash.
pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|e| format!("stored password hash is corrupt: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Tauri-managed state: whether the app is currently unlocked. Starts
/// `true` when no password has been set yet (nothing to protect), `false`
/// otherwise — flipped by `commands::auth::auth_unlock` on a correct
/// password and by `auth_lock` (or app restart) back to locked.
pub struct AuthState(pub Mutex<bool>);

impl AuthState {
    pub fn new(unlocked: bool) -> Self {
        Self(Mutex::new(unlocked))
    }

    pub fn is_unlocked(&self) -> bool {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_unlocked(&self, value: bool) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = value;
    }
}

/// Call as the first line of every command that reads or writes note data.
/// Returns the same shape as every other command's error (`String`) so `?`
/// composes directly at each call site.
pub fn require_unlocked(state: &tauri::State<'_, std::sync::Arc<AuthState>>) -> Result<(), String> {
    if state.is_unlocked() {
        Ok(())
    } else {
        Err("locked: unlock the app before doing this".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_round_trips() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(!verify_password("wrong password", &hash).unwrap());
    }

    #[test]
    fn verify_errors_on_corrupt_hash() {
        assert!(verify_password("anything", "not a real phc hash").is_err());
    }

    #[test]
    fn two_hashes_of_the_same_password_differ() {
        // Different random salts each time — this is what makes rainbow
        // tables useless, and it's the whole reason to use a salted KDF
        // instead of a bare hash.
        let a = hash_password("same password").unwrap();
        let b = hash_password("same password").unwrap();
        assert_ne!(a, b);
        assert!(verify_password("same password", &a).unwrap());
        assert!(verify_password("same password", &b).unwrap());
    }

    #[test]
    fn auth_state_starts_at_given_value_and_toggles() {
        let state = AuthState::new(false);
        assert!(!state.is_unlocked());
        state.set_unlocked(true);
        assert!(state.is_unlocked());
        state.set_unlocked(false);
        assert!(!state.is_unlocked());
    }
}
