use std::fmt;
use std::io;

#[derive(Debug)]
pub enum LegionError {
    Io(io::Error),
    PermissionDenied {
        path: String,
        action: &'static str,
    },
    DeviceNotFound(String),
    UnsupportedFeature(String),
    InvalidValue(String),
    ParseError(String),
}

impl fmt::Display for LegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LegionError::Io(err) => write!(f, "I/O error: {}", err),
            LegionError::PermissionDenied { path, action } => {
                write!(
                    f,
                    "Permission denied while trying to {} '{}'.\n\
                     Ensure your user has access or install the legion-cli udev rules:\n\
                     sudo cp udev/99-lenovo-legion.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules && sudo udevadm trigger",
                    action, path
                )
            }
            LegionError::DeviceNotFound(msg) => write!(f, "Device not found: {}", msg),
            LegionError::UnsupportedFeature(msg) => write!(f, "Feature not supported: {}", msg),
            LegionError::InvalidValue(msg) => write!(f, "Invalid parameter: {}", msg),
            LegionError::ParseError(msg) => write!(f, "Parsing error: {}", msg),
        }
    }
}

impl std::error::Error for LegionError {}

impl From<io::Error> for LegionError {
    fn from(err: io::Error) -> Self {
        LegionError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, LegionError>;

