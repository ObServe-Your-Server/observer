use std::panic::Location;
use getset::Getters;

#[derive(Debug, Clone, PartialEq, Getters)]
pub struct CallSite {
    #[getset(get = "pub")]
    file: String,
    #[getset(get = "pub")]
    line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    LOW,
    MEDIUM,
    FATAL,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::LOW => write!(f, "low"),
            Severity::MEDIUM => write!(f, "medium"),
            Severity::FATAL => write!(f, "fatal"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Getters)]
pub struct Error {
    #[getset(get = "pub")]
    severity: Severity,
    #[getset(get = "pub")]
    call_site: CallSite,
    #[getset(get = "pub")]
    message: String,
}

impl Error {
    #[track_caller]
    pub fn new(message: &str, severity: Severity) -> Error {
        let loc = Location::caller();
        Error {
            call_site: CallSite {
                file: loc.file().to_string(),
                line: loc.line(),
            },
            severity,
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error_handling::error::Severity;

    #[test]
    fn basic_error_creation() {
        let err = crate::error_handling::error::Error::new("Test error Message", Severity::LOW);
        println!("{:?}", err);
        assert_eq!(
            err.call_site.file,
            "bins/client/src/error_handling/error.rs"
        );
        assert_eq!(err.message, "Test error Message");
        assert_eq!(err.severity, Severity::LOW);
    }
}
