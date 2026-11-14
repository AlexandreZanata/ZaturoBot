//! Typed Linux USB ADB adapter. No arbitrary shell, AI transport or image persistence.
mod adb;
mod session;

pub use adb::{AdbClient, AdbLimits, SensitiveFrame, parse_devices};
pub use session::{SessionLease, SessionManager};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid Android configuration or typed argument")]
    Invalid,
    #[error("ADB executable is unavailable or cannot be started")]
    Unavailable,
    #[error(
        "ADB process could not be started; operating-system error details contain no device output"
    )]
    SpawnFailed {
        kind: std::io::ErrorKind,
        os_code: Option<i32>,
    },
    #[error("ADB command timed out")]
    Timeout,
    #[error("ADB output exceeded its limit")]
    OutputLimit,
    #[error("ADB returned invalid bounded output")]
    InvalidOutput,
    #[error("ADB command failed; raw device output is not logged")]
    CommandFailed,
    #[error("the selected registered USB device is disconnected")]
    Disconnected,
    #[error("the device requires physical USB authorization or USB permission")]
    Unauthorized,
    #[error("Android operation is not authorized by this active lease")]
    PermissionDenied,
    #[error("Android device already has an active control lease")]
    Busy,
    #[error("Android session registry is unavailable")]
    SessionUnavailable,
}
pub type Result<T> = std::result::Result<T, Error>;
