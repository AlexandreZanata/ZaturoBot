//! Pure USB Android registration, observation and bounded sensitive-view contracts.
use serde::{Deserialize, Serialize};

pub const MAX_ANDROID_DEVICES: usize = 8;
pub const MAX_ANDROID_SCREENSHOT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ANDROID_LEASE_SECONDS: u64 = 300;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AndroidDeviceState {
    Disconnected,
    Connecting,
    Unauthorized,
    Ready,
    Busy,
    Error,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AndroidAuthorization {
    Unknown,
    Required,
    Authorized,
    UsbPermissionDenied,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AndroidTransport {
    Usb,
    Network,
    Emulator,
    Unknown,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AndroidCapability {
    DeviceStatus,
    SensitiveScreenView,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AndroidControlActor {
    Owner,
    Agent,
    TestRunner,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegisteredAndroidDevice {
    pub device_id: String,
    pub adb_serial: String,
    pub display_name: Option<String>,
    pub capabilities: Vec<AndroidCapability>,
}
impl RegisteredAndroidDevice {
    pub fn validate(&self) -> Result<(), &'static str> {
        crate::validate_identifier(&self.device_id).map_err(|_| "invalid Android device ID")?;
        if !valid_usb_serial(&self.adb_serial) {
            return Err("an explicit USB serial is required");
        }
        if self
            .display_name
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() > 80 || v.chars().any(char::is_control))
        {
            return Err("invalid Android display name");
        }
        if self.capabilities.is_empty()
            || self.capabilities.len() > 2
            || self
                .capabilities
                .iter()
                .enumerate()
                .any(|(i, c)| self.capabilities[..i].contains(c))
        {
            return Err("invalid Android capabilities");
        }
        Ok(())
    }
}
/// USB registrations do not implicitly authorize ADB TCP/IP, emulator or command selectors.
pub fn valid_usb_serial(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && !value.starts_with("emulator-")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidDeviceObservation {
    pub adb_serial: String,
    pub model: Option<String>,
    pub transport: AndroidTransport,
    pub state: AndroidDeviceState,
    pub authorization: AndroidAuthorization,
    pub last_seen_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidDeviceStatus {
    pub device_id: String,
    pub adb_serial: String,
    pub display_name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub android_version: Option<String>,
    pub api_level: Option<u32>,
    pub state: AndroidDeviceState,
    pub authorization: AndroidAuthorization,
    pub last_seen_at: Option<i64>,
    pub capabilities: Vec<AndroidCapability>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidSessionStatus {
    pub device_id: String,
    pub actor: AndroidControlActor,
    pub expires_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidWatchRequest {
    pub duration_seconds: u64,
    pub poll_seconds: u64,
}
impl AndroidWatchRequest {
    pub fn validate(&self) -> bool {
        (5..=MAX_ANDROID_LEASE_SECONDS).contains(&self.duration_seconds)
            && (2..=15).contains(&self.poll_seconds)
            && self.poll_seconds <= self.duration_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usb_registration_and_watch_are_explicit_and_bounded() {
        for serial in [
            "",
            "-s",
            "a b",
            "127.0.0.1:5555",
            "emulator-5554",
            "serial;command",
        ] {
            assert!(!valid_usb_serial(serial));
        }
        assert!(valid_usb_serial("USB_SYNTHETIC-01"));
        let mut r = RegisteredAndroidDevice {
            device_id: "owner-phone".into(),
            adb_serial: "USB_SYNTHETIC-01".into(),
            display_name: Some("Owner phone".into()),
            capabilities: vec![AndroidCapability::DeviceStatus],
        };
        r.validate().unwrap();
        r.capabilities.push(AndroidCapability::DeviceStatus);
        assert!(r.validate().is_err());
        assert!(
            AndroidWatchRequest {
                duration_seconds: 30,
                poll_seconds: 5
            }
            .validate()
        );
        assert!(
            !AndroidWatchRequest {
                duration_seconds: 301,
                poll_seconds: 5
            }
            .validate()
        );
        assert!(
            !AndroidWatchRequest {
                duration_seconds: 30,
                poll_seconds: 0
            }
            .validate()
        );
    }
}
