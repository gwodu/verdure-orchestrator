//! Offline licence tokens (knowledge/26 §4.6). The SDK never phones home to
//! check a licence: the token is signed by the issuer's Ed25519 key, the
//! verifying key is compiled into the vendor's build, and everything needed
//! to decide is in the token. Format: `vl1.<payload-b64>.<signature-b64>`
//! where the payload is the JSON of [`LicenseClaims`] and the signature
//! covers those exact bytes.

use serde::{Deserialize, Serialize};

use crate::signing::{self, KeyPair};
use crate::VError;

const PREFIX: &str = "vl1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub vendor_id: String,
    /// Seat cap, 0 = unlimited (a government fleet, `[OQ-105]`).
    #[serde(default)]
    pub seats: u32,
    pub issued_at: u64,
    /// Unix seconds; 0 = never expires.
    #[serde(default)]
    pub expires_at: u64,
    /// Bundle ids this licence may download and run; `"*"` = any.
    #[serde(default)]
    pub bundles: Vec<String>,
    /// Model licences the vendor has acknowledged (e.g. "llama-community",
    /// "gemma-terms"); a bundle needing one the token lacks is refused.
    #[serde(default)]
    pub acknowledgements: Vec<String>,
}

impl LicenseClaims {
    pub fn entitles(&self, bundle_id: &str) -> bool {
        self.bundles.iter().any(|b| b == "*" || b == bundle_id)
    }
}

/// Mints tokens. Lives in the CLI and in tests; a vendor build never holds a
/// signing key.
pub struct Issuer {
    key: KeyPair,
}

impl Issuer {
    pub fn new(key: KeyPair) -> Self {
        Issuer { key }
    }
    pub fn public_key(&self) -> String {
        self.key.public_b64()
    }
    pub fn issue(&self, claims: &LicenseClaims) -> String {
        let payload = serde_json::to_vec(claims).expect("claims serialise");
        format!(
            "{PREFIX}.{}.{}",
            signing::b64(&payload),
            self.key.sign(&payload)
        )
    }
}

/// Checks tokens against one issuer public key.
pub struct Verifier {
    public_key: String,
}

impl Verifier {
    pub fn new(public_key_b64: &str) -> Self {
        Verifier {
            public_key: public_key_b64.trim().to_string(),
        }
    }

    /// Verify signature, format and expiry (against `now`, Unix seconds).
    pub fn verify(&self, token: &str, now: u64) -> Result<LicenseClaims, VError> {
        let mut parts = token.trim().split('.');
        let (Some(prefix), Some(payload), Some(sig), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(VError::License(
                "the key is not in the expected form".into(),
            ));
        };
        if prefix != PREFIX {
            return Err(VError::License(format!("unknown key version {prefix}")));
        }
        let payload =
            signing::unb64(payload).map_err(|e| VError::License(format!("unreadable key: {e}")))?;
        signing::verify(&self.public_key, &payload, sig)
            .map_err(|e| VError::License(format!("the key was not issued for this build: {e}")))?;
        let claims: LicenseClaims = serde_json::from_slice(&payload)
            .map_err(|e| VError::License(format!("unreadable key contents: {e}")))?;
        if claims.expires_at != 0 && now >= claims.expires_at {
            return Err(VError::License("the key has expired".into()));
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> LicenseClaims {
        LicenseClaims {
            vendor_id: "nabla".into(),
            seats: 500,
            issued_at: 1_000,
            expires_at: 2_000,
            bundles: vec!["scribe-P2".into(), "scribe-L2".into()],
            acknowledgements: vec!["cc-by-parakeet".into()],
        }
    }

    #[test]
    fn a_token_verifies_with_the_issuer_key_only() {
        let issuer = Issuer::new(KeyPair::generate().unwrap());
        let token = issuer.issue(&claims());
        assert!(token.starts_with("vl1."));
        let got = Verifier::new(&issuer.public_key())
            .verify(&token, 1_500)
            .unwrap();
        assert_eq!(got, claims());

        let stranger = Issuer::new(KeyPair::generate().unwrap());
        let err = Verifier::new(&stranger.public_key())
            .verify(&token, 1_500)
            .unwrap_err();
        assert!(matches!(err, VError::License(m) if m.contains("not issued for this build")));
    }

    #[test]
    fn tampering_with_the_claims_breaks_the_signature() {
        let issuer = Issuer::new(KeyPair::generate().unwrap());
        let token = issuer.issue(&claims());
        let mut parts: Vec<&str> = token.split('.').collect();
        let mut bigger = claims();
        bigger.seats = 50_000;
        let forged = signing::b64(&serde_json::to_vec(&bigger).unwrap());
        parts[1] = &forged;
        let forged_token = parts.join(".");
        assert!(Verifier::new(&issuer.public_key())
            .verify(&forged_token, 1_500)
            .is_err());
    }

    #[test]
    fn expiry_and_shape_are_enforced() {
        let issuer = Issuer::new(KeyPair::generate().unwrap());
        let v = Verifier::new(&issuer.public_key());
        let token = issuer.issue(&claims());
        assert_eq!(
            v.verify(&token, 2_000).unwrap_err(),
            VError::License("the key has expired".into())
        );
        let forever = issuer.issue(&LicenseClaims {
            expires_at: 0,
            ..claims()
        });
        assert!(v.verify(&forever, u64::MAX).is_ok());
        assert!(v.verify("garbage", 0).is_err());
        assert!(v.verify("vl2.a.b", 0).is_err());
        assert!(v.verify("vl1.a.b.c", 0).is_err());
    }

    #[test]
    fn entitlement_is_by_id_or_wildcard() {
        assert!(claims().entitles("scribe-P2"));
        assert!(!claims().entitles("tutor-L2"));
        let any = LicenseClaims {
            bundles: vec!["*".into()],
            ..claims()
        };
        assert!(any.entitles("tutor-L2"));
    }
}
