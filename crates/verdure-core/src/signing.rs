//! Ed25519 detached signatures, the one primitive behind licence tokens and
//! signed manifests. Keys and signatures travel as URL-safe base64 without
//! padding, so they paste into a config file or a shell without quoting.

use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

pub fn b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

pub fn unb64(s: &str) -> Result<Vec<u8>, String> {
    B64.decode(s.trim()).map_err(|e| format!("bad base64: {e}"))
}

/// A signing key with its public half. Generated with the OS's randomness;
/// the secret never leaves the process that generated it unless `secret_b64`
/// is called (the CLI's `license keygen` prints it once).
pub struct KeyPair {
    signing: SigningKey,
}

impl KeyPair {
    pub fn generate() -> Result<KeyPair, String> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|e| format!("no randomness available: {e}"))?;
        Ok(KeyPair {
            signing: SigningKey::from_bytes(&seed),
        })
    }

    pub fn from_secret_b64(s: &str) -> Result<KeyPair, String> {
        let bytes = unb64(s)?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "a secret key is 32 bytes".to_string())?;
        Ok(KeyPair {
            signing: SigningKey::from_bytes(&seed),
        })
    }

    pub fn secret_b64(&self) -> String {
        b64(&self.signing.to_bytes())
    }

    pub fn public_b64(&self) -> String {
        b64(self.signing.verifying_key().as_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> String {
        b64(&self.signing.sign(message).to_bytes())
    }
}

/// Verify `signature_b64` over `message` with the public key `public_b64`.
pub fn verify(public_b64: &str, message: &[u8], signature_b64: &str) -> Result<(), String> {
    let pk: [u8; 32] = unb64(public_b64)?
        .try_into()
        .map_err(|_| "a public key is 32 bytes".to_string())?;
    let key = VerifyingKey::from_bytes(&pk).map_err(|e| format!("bad public key: {e}"))?;
    let sig =
        Signature::from_slice(&unb64(signature_b64)?).map_err(|e| format!("bad signature: {e}"))?;
    key.verify(message, &sig)
        .map_err(|_| "signature does not match".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_and_reject_tampering() {
        let kp = KeyPair::generate().unwrap();
        let sig = kp.sign(b"hello");
        assert!(verify(&kp.public_b64(), b"hello", &sig).is_ok());
        assert!(verify(&kp.public_b64(), b"hellp", &sig).is_err());
        let other = KeyPair::generate().unwrap();
        assert!(verify(&other.public_b64(), b"hello", &sig).is_err());
        assert!(verify("not-a-key", b"hello", &sig).is_err());
        assert!(verify(&kp.public_b64(), b"hello", "zz").is_err());
    }

    #[test]
    fn a_secret_round_trips_and_keeps_its_public_key() {
        let kp = KeyPair::generate().unwrap();
        let again = KeyPair::from_secret_b64(&kp.secret_b64()).unwrap();
        assert_eq!(again.public_b64(), kp.public_b64());
        assert_eq!(kp.public_b64().len(), 43, "32 bytes, unpadded base64");
    }
}
