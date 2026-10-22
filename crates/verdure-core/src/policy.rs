//! What the runtime may do on the vendor's behalf. The defaults are the
//! privacy posture of knowledge/06 written as data: nothing leaves the device
//! except a bundle download the vendor configured, nothing is measured, nothing
//! is kept past the session, and the cloud is off until the vendor wires it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkPolicy {
    /// No socket, ever. Bundles must already be on the device.
    None,
    /// Only `verdure-download`, only to hosts the manifest/policy lists.
    DownloadsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Telemetry {
    Off,
    /// PHI-free counters (latency, RTF, OOM events, bundle id) written to a
    /// local file the *vendor* may upload. Verdure never uploads anything.
    LocalCountersOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Storage {
    /// Transcripts, indexes and caches encrypted with a per-install key.
    EncryptedAppSandbox,
    /// Plain files in the app sandbox (development, or a vendor that
    /// encrypts the whole volume already).
    PlainAppSandbox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Retention {
    /// Nothing survives `delete(Session)` / process exit.
    SessionOnly,
    Days(u32),
    Indefinite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fallback {
    Disabled,
    /// A `CloudFallback` the vendor implements; Verdure calls *into* it and
    /// never opens a socket itself.
    VendorHook,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub network: NetworkPolicy,
    pub telemetry: Telemetry,
    pub storage: Storage,
    pub retention: Retention,
    pub fallback: Fallback,
    /// Hosts downloads may contact (exact host names). The manifest's own host
    /// is implied once the manifest is trusted; this list is what admits the
    /// manifest URL itself. Empty + `DownloadsOnly` = any https host, which is
    /// the right default for a vendor pointing at their own CDN.
    pub allowed_hosts: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            network: NetworkPolicy::DownloadsOnly,
            telemetry: Telemetry::Off,
            storage: Storage::EncryptedAppSandbox,
            retention: Retention::SessionOnly,
            fallback: Fallback::Disabled,
            allowed_hosts: Vec::new(),
        }
    }
}

impl Policy {
    /// May the downloader contact `url`? The one place network permission is
    /// decided, so `verdure-download` asks and never reasons about it itself.
    pub fn permits_download(&self, url: &str) -> Result<(), crate::VError> {
        if self.network == NetworkPolicy::None {
            return Err(crate::VError::PolicyDenied("network is off".into()));
        }
        let rest = url.strip_prefix("https://").ok_or_else(|| {
            crate::VError::PolicyDenied(format!("only https downloads are allowed, not {url}"))
        })?;
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = host.rsplit('@').next().unwrap_or(host); // no userinfo tricks
        let host = host.split(':').next().unwrap_or(host);
        if host.is_empty() {
            return Err(crate::VError::PolicyDenied(format!("no host in {url}")));
        }
        if !self.allowed_hosts.is_empty()
            && !self
                .allowed_hosts
                .iter()
                .any(|h| h.eq_ignore_ascii_case(host))
        {
            return Err(crate::VError::PolicyDenied(format!(
                "{host} is not an allowed download host"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_policy_is_the_privacy_posture() {
        let p = Policy::default();
        assert_eq!(p.network, NetworkPolicy::DownloadsOnly);
        assert_eq!(p.telemetry, Telemetry::Off);
        assert_eq!(p.storage, Storage::EncryptedAppSandbox);
        assert_eq!(p.retention, Retention::SessionOnly);
        assert_eq!(p.fallback, Fallback::Disabled);
    }

    #[test]
    fn downloads_are_https_only_and_host_gated() {
        let open = Policy::default();
        assert!(open
            .permits_download("https://cdn.example.org/m.gguf")
            .is_ok());
        assert!(open
            .permits_download("http://cdn.example.org/m.gguf")
            .is_err());
        assert!(open.permits_download("https:///nohost").is_err());

        let gated = Policy {
            allowed_hosts: vec!["cdn.vendor.com".into()],
            ..Policy::default()
        };
        assert!(gated.permits_download("https://CDN.vendor.com/x").is_ok());
        assert!(gated
            .permits_download("https://cdn.vendor.com:443/x")
            .is_ok());
        assert!(gated.permits_download("https://evil.com/x").is_err());
        // Userinfo and path tricks don't smuggle a host past the check.
        assert!(gated
            .permits_download("https://cdn.vendor.com@evil.com/x")
            .is_err());
        assert!(gated
            .permits_download("https://evil.com/cdn.vendor.com")
            .is_err());

        let off = Policy {
            network: NetworkPolicy::None,
            ..Policy::default()
        };
        assert_eq!(
            off.permits_download("https://cdn.vendor.com/x"),
            Err(crate::VError::PolicyDenied("network is off".into()))
        );
    }

    #[test]
    fn policy_round_trips_through_json() {
        let p = Policy {
            retention: Retention::Days(14),
            ..Policy::default()
        };
        let back: Policy = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back, p);
    }
}
