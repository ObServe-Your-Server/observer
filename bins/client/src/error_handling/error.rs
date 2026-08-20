use crate::config::ServerConfig;
use std::panic::Location;

#[derive(Debug, Clone, PartialEq)]
pub struct CallSite {
    file: String,
    line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    LOW,
    MEDIUM,
    FATAL,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    severity: Severity,
    call_site: CallSite,
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
