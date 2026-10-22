//! Encryption at rest (knowledge/26 §4.6). Transcripts, indexes and caches
//! are written through here as AES-256-GCM files under one per-install key;
//! `delete(All)` destroys the key first (crypto-shred), so even a file the
//! unlink misses is noise afterwards.
//!
//! Where the key lives is the `KeyStore` port. v0.1 ships `FileKeyStore`: a
//! 32-byte file with owner-only permissions inside the app sandbox, which is
//! as strong as the sandbox and no stronger. The platform key stores
//! (Keychain, Keystore, DPAPI, libsecret) are the second implementation and
//! need platform code — `[OQ-110]`. The vendor's compliance pack must say
//! which one a build uses.

use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use verdure_core::VError;

const MAGIC: &[u8; 4] = b"VV1\0";
const NONCE_LEN: usize = 12;

/// Where the per-install key is kept.
pub trait KeyStore: Send + Sync {
    /// The key, creating a fresh random one the first time.
    fn load_or_create(&self) -> Result<[u8; 32], VError>;
    /// Destroy the key beyond recovery. Idempotent.
    fn destroy(&self) -> Result<(), VError>;
}

/// A key file in the app sandbox, owner-only on Unix.
pub struct FileKeyStore {
    path: PathBuf,
}

impl FileKeyStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        FileKeyStore { path: path.into() }
    }
}

impl KeyStore for FileKeyStore {
    fn load_or_create(&self) -> Result<[u8; 32], VError> {
        if let Ok(bytes) = fs::read(&self.path) {
            return bytes
                .try_into()
                .map_err(|_| VError::Storage("the key file is damaged".into()));
        }
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).map_err(|e| VError::Storage(format!("no randomness: {e}")))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| VError::Storage(e.to_string()))?;
        }
        fs::write(&self.path, key).map_err(|e| VError::Storage(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))
                .map_err(|e| VError::Storage(e.to_string()))?;
        }
        Ok(key)
    }

    fn destroy(&self) -> Result<(), VError> {
        if self.path.exists() {
            // Overwrite before unlinking so a recovered inode holds zeros.
            fs::write(&self.path, [0u8; 32]).ok();
            fs::remove_file(&self.path).map_err(|e| VError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}

/// Encrypted files in one directory.
pub struct Vault {
    dir: PathBuf,
    cipher: Aes256Gcm,
    keystore: Box<dyn KeyStore>,
}

impl Vault {
    pub fn open(dir: impl Into<PathBuf>, keystore: Box<dyn KeyStore>) -> Result<Vault, VError> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(|e| VError::Storage(e.to_string()))?;
        let key = keystore.load_or_create()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        Ok(Vault {
            dir,
            cipher,
            keystore,
        })
    }

    fn path_for(&self, name: &str) -> Result<PathBuf, VError> {
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(VError::InvalidInput(format!(
                "bad vault entry name {name:?}"
            )));
        }
        Ok(self.dir.join(format!("{name}.vv")))
    }

    /// Encrypt and write `plaintext` as `name`, replacing any previous entry.
    /// The name is bound into the ciphertext, so a file renamed on disk
    /// fails to open under its new name.
    pub fn write(&self, name: &str, plaintext: &[u8]) -> Result<(), VError> {
        let path = self.path_for(name)?;
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|e| VError::Storage(format!("no randomness: {e}")))?;
        let ct = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: name.as_bytes(),
                },
            )
            .map_err(|_| VError::Storage("encryption failed".into()))?;
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ct.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        let tmp = path.with_extension("vv.tmp");
        fs::write(&tmp, &out).map_err(|e| VError::Storage(e.to_string()))?;
        fs::rename(&tmp, &path).map_err(|e| VError::Storage(e.to_string()))
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>, VError> {
        let path = self.path_for(name)?;
        let bytes = fs::read(&path).map_err(|e| VError::Storage(format!("{name}: {e}")))?;
        if bytes.len() < MAGIC.len() + NONCE_LEN || &bytes[..MAGIC.len()] != MAGIC {
            return Err(VError::Storage(format!("{name} is not a vault file")));
        }
        let nonce = &bytes[MAGIC.len()..MAGIC.len() + NONCE_LEN];
        let ct = &bytes[MAGIC.len() + NONCE_LEN..];
        self.cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ct,
                    aad: name.as_bytes(),
                },
            )
            .map_err(|_| {
                VError::Storage(format!("{name} failed to decrypt (wrong key or tampered)"))
            })
    }

    pub fn exists(&self, name: &str) -> bool {
        self.path_for(name).map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn delete(&self, name: &str) -> Result<(), VError> {
        let path = self.path_for(name)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(VError::Storage(e.to_string())),
        }
    }

    pub fn list(&self) -> Result<Vec<String>, VError> {
        let mut names: Vec<String> = fs::read_dir(&self.dir)
            .map_err(|e| VError::Storage(e.to_string()))?
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".vv").map(str::to_string))
            .collect();
        names.sort();
        Ok(names)
    }

    /// Crypto-shred: destroy the key, then unlink every entry. Consumes the
    /// vault — nothing can be read through it afterwards.
    pub fn shred(self) -> Result<(), VError> {
        self.keystore.destroy()?;
        for name in self.list()? {
            self.delete(&name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(dir: &std::path::Path) -> Vault {
        Vault::open(
            dir.join("vault"),
            Box::new(FileKeyStore::new(dir.join("key"))),
        )
        .unwrap()
    }

    #[test]
    fn round_trips_and_lists() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        v.write("transcript-1", b"patient reports a cough").unwrap();
        v.write("notes", b"").unwrap();
        assert_eq!(v.read("transcript-1").unwrap(), b"patient reports a cough");
        assert_eq!(v.read("notes").unwrap(), b"");
        assert_eq!(v.list().unwrap(), vec!["notes", "transcript-1"]);
        assert!(v.exists("notes") && !v.exists("nope"));
        v.delete("notes").unwrap();
        v.delete("notes").unwrap(); // idempotent
        assert_eq!(v.list().unwrap(), vec!["transcript-1"]);
    }

    #[test]
    fn nothing_readable_sits_on_disk_and_tampering_is_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        v.write("t", b"the secret words").unwrap();
        let path = tmp.path().join("vault").join("t.vv");
        let raw = fs::read(&path).unwrap();
        assert!(
            !raw.windows(6).any(|w| w == b"secret"),
            "plaintext leaked to disk"
        );
        let mut flipped = raw.clone();
        let last = flipped.len() - 1;
        flipped[last] ^= 1;
        fs::write(&path, &flipped).unwrap();
        assert!(matches!(v.read("t"), Err(VError::Storage(m)) if m.contains("tampered")));
        // Renaming the file on disk breaks it too: the name is authenticated.
        fs::write(&path, &raw).unwrap();
        fs::rename(&path, tmp.path().join("vault").join("u.vv")).unwrap();
        assert!(v.read("u").is_err());
    }

    #[test]
    fn the_key_persists_across_opens_and_shred_makes_everything_noise() {
        let tmp = tempfile::tempdir().unwrap();
        vault(tmp.path()).write("t", b"kept").unwrap();
        let again = vault(tmp.path());
        assert_eq!(again.read("t").unwrap(), b"kept");
        let raw = fs::read(tmp.path().join("vault").join("t.vv")).unwrap();
        again.shred().unwrap();
        assert!(!tmp.path().join("key").exists());
        assert_eq!(vault(tmp.path()).list().unwrap(), Vec::<String>::new());
        // Even if the old bytes were recovered, the new key cannot open them.
        fs::write(tmp.path().join("vault").join("t.vv"), raw).unwrap();
        assert!(vault(tmp.path()).read("t").is_err());
    }

    #[test]
    fn entry_names_cannot_escape_the_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        for bad in ["../x", "a/b", "", ".", ".."] {
            assert!(
                matches!(v.write(bad, b"x"), Err(VError::InvalidInput(_))),
                "{bad:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_key_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let _ = vault(tmp.path());
        let mode = fs::metadata(tmp.path().join("key"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
