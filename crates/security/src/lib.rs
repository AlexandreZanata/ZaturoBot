//! Owner/device authentication and the narrow foundation permission policy.
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

#[derive(thiserror::Error, Debug)]
#[error(
    "endpoint must be an HTTPS origin, or a literal loopback HTTP origin, without user information, query or fragment"
)]
pub struct InvalidEndpoint;

pub fn validate_endpoint(value: &str) -> Result<(), InvalidEndpoint> {
    let url = url::Url::parse(value).map_err(|_| InvalidEndpoint)?;
    let origin_only = url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path() == "/";
    let secure = url.scheme() == "https"
        || (url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "::1")));
    if origin_only && secure && url.host_str().is_some() {
        Ok(())
    } else {
        Err(InvalidEndpoint)
    }
}

pub fn private_bind(address: std::net::IpAddr) -> bool {
    match address {
        std::net::IpAddr::V4(ip) => {
            ip.is_loopback()
                || ip.is_private()
                || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        std::net::IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
    }
}

#[derive(thiserror::Error, Debug)]
#[error("credential must contain at least 32 printable non-whitespace ASCII characters")]
pub struct InvalidCredential;

/// Keeps only a digest; neither Debug nor Display exposes a credential.
#[derive(Clone)]
pub struct Credential([u8; 32]);
impl Credential {
    pub fn new(value: &str) -> Result<Self, InvalidCredential> {
        if value.len() < 32 || value.len() > 256 || !value.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(InvalidCredential);
        }
        Ok(Self(Sha256::digest(value.as_bytes()).into()))
    }
    pub fn verifies(&self, bearer: &str) -> bool {
        let candidate: [u8; 32] = Sha256::digest(bearer.as_bytes()).into();
        bool::from(self.0.ct_eq(&candidate))
    }
}
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credential([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionClass {
    ReadOnly,
    LocalWrite,
    GitWrite,
    ExternalCommunication,
    AccountModification,
    Destructive,
    Financial,
    Privileged,
}

/// Phase one implements exactly one read-only tool. Other classes fail closed.
pub fn permits(tool: &str, action: ActionClass, capabilities: &[String]) -> bool {
    tool == "diagnostic.v1"
        && action == ActionClass::ReadOnly
        && capabilities.iter().any(|c| c == tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_verify_and_debug_redacts() {
        let raw = "synthetic-only-credential-with-32-characters";
        let credential = Credential::new(raw).unwrap();
        assert!(credential.verifies(raw));
        assert!(!credential.verifies("wrong"));
        assert!(!format!("{credential:?}").contains(raw));
        assert!(Credential::new("short").is_err());
        assert!(Credential::new("has whitespace with otherwise sufficient length").is_err());
    }
    #[test]
    fn untrusted_proposals_cannot_grant_permissions() {
        let caps = vec!["diagnostic.v1".into(), "root".into()];
        assert!(permits("diagnostic.v1", ActionClass::ReadOnly, &caps));
        for class in [
            ActionClass::LocalWrite,
            ActionClass::GitWrite,
            ActionClass::ExternalCommunication,
            ActionClass::AccountModification,
            ActionClass::Destructive,
            ActionClass::Financial,
            ActionClass::Privileged,
        ] {
            assert!(!permits("diagnostic.v1", class, &caps));
        }
        assert!(!permits("terminal.execute", ActionClass::ReadOnly, &caps));
        assert!(!permits("diagnostic.v1", ActionClass::ReadOnly, &[]));
    }
    #[test]
    fn transport_policy_rejects_public_binds_and_insecure_origins() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "100.100.100.100",
            "::1",
            "fd7a:115c:a1e0::1",
        ] {
            assert!(private_bind(ip.parse().unwrap()));
        }
        for ip in ["0.0.0.0", "8.8.8.8", "::", "2001:4860:4860::8888"] {
            assert!(!private_bind(ip.parse().unwrap()));
        }
        assert!(validate_endpoint("http://127.0.0.1:7878").is_ok());
        assert!(validate_endpoint("https://hub.example").is_ok());
        for url in [
            "http://hub.example",
            "http://127.0.0.1.evil.example",
            "https://user:secret@hub.example",
            "https://hub.example/path",
            "https://hub.example?secret=value",
        ] {
            assert!(validate_endpoint(url).is_err());
        }
    }
}
