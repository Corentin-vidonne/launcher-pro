//! Coffre des jeux masqués : mot de passe haché avec Argon2id (sel aléatoire, format PHC).
//! Seule l'empreinte est enregistrée ; le mot de passe lui-même n'est jamais écrit sur disque.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Longueur minimale acceptée (en caractères).
pub const MIN_LEN: usize = 4;

pub fn hash(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Impossible de protéger le mot de passe : {e}"))
}

/// Comparaison en temps constant, assurée par Argon2.
pub fn verify(password: &str, phc: &str) -> bool {
    PasswordHash::new(phc)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify() {
        let phc = hash("mon-coffre-secret").unwrap();
        assert!(phc.starts_with("$argon2id$"));
        assert!(!phc.contains("mon-coffre-secret"));
        assert!(verify("mon-coffre-secret", &phc));
        assert!(!verify("mon-coffre-secreT", &phc));
        assert!(!verify("", &phc));
        // Deux empreintes du même mot de passe diffèrent (sel aléatoire).
        assert_ne!(phc, hash("mon-coffre-secret").unwrap());
    }

    #[test]
    fn rejects_garbage_hash() {
        assert!(!verify("1234", "pas-une-empreinte"));
    }
}
