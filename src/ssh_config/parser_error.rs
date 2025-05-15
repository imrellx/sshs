#[derive(Debug)]
pub struct UnknownEntryError {
    pub line: String,
    pub entry: String,
}

impl std::fmt::Display for UnknownEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown entry '{}' on line: {}", self.entry, self.line)
    }
}

impl std::error::Error for UnknownEntryError {}

#[derive(Debug)]
pub enum InvalidIncludeErrorDetails {
    Pattern(glob::PatternError),
    Glob(glob::GlobError),
    Io(std::io::Error),
    HostsInsideHostBlock,
}

impl std::fmt::Display for InvalidIncludeErrorDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidIncludeErrorDetails::Pattern(e) => write!(f, "Invalid glob pattern: {}", e),
            InvalidIncludeErrorDetails::Glob(e) => write!(f, "Glob matching error: {}", e),
            InvalidIncludeErrorDetails::Io(e) => write!(f, "IO error during include: {}", e),
            InvalidIncludeErrorDetails::HostsInsideHostBlock => {
                write!(f, "Host definitions found inside host block (not allowed)")
            }
        }
    }
}

impl std::error::Error for InvalidIncludeErrorDetails {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InvalidIncludeErrorDetails::Pattern(e) => Some(e),
            InvalidIncludeErrorDetails::Glob(e) => Some(e),
            InvalidIncludeErrorDetails::Io(e) => Some(e),
            InvalidIncludeErrorDetails::HostsInsideHostBlock => None,
        }
    }
}

#[derive(Debug)]
pub struct InvalidIncludeError {
    pub line: String,
    pub details: InvalidIncludeErrorDetails,
}

impl std::fmt::Display for InvalidIncludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid include directive on line '{}': {}", self.line, self.details)
    }
}

impl std::error::Error for InvalidIncludeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.details)
    }
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    UnparseableLine(String),
    UnknownEntry(UnknownEntryError),
    InvalidInclude(InvalidIncludeError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "IO error: {}", e),
            ParseError::UnparseableLine(line) => write!(f, "Unable to parse line: '{}'", line),
            ParseError::UnknownEntry(e) => write!(f, "{}", e),
            ParseError::InvalidInclude(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Io(e) => Some(e),
            ParseError::UnparseableLine(_) => None,
            ParseError::UnknownEntry(e) => Some(e),
            ParseError::InvalidInclude(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::Io(e)
    }
}

impl From<UnknownEntryError> for ParseError {
    fn from(e: UnknownEntryError) -> Self {
        ParseError::UnknownEntry(e)
    }
}

impl From<InvalidIncludeError> for ParseError {
    fn from(e: InvalidIncludeError) -> Self {
        ParseError::InvalidInclude(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_unknown_entry_error_display() {
        let error = UnknownEntryError {
            line: "SomeUnknownOption value".to_string(),
            entry: "SomeUnknownOption".to_string(),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Unknown entry 'SomeUnknownOption'"));
        assert!(display.contains("SomeUnknownOption value"));
    }
    
    #[test]
    fn test_invalid_include_error_display() {
        let error = InvalidIncludeError {
            line: "Include /bad/path".to_string(),
            details: InvalidIncludeErrorDetails::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found"
            )),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Invalid include directive"));
        assert!(display.contains("/bad/path"));
        assert!(display.contains("IO error during include"));
    }
    
    #[test]
    fn test_parse_error_implements_error_trait() {
        // Test that all ParseError variants properly implement Error
        let io_error = ParseError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test error"
        ));
        
        // Should be able to use as Error trait object
        let error: &dyn Error = &io_error;
        assert!(error.source().is_some());
        
        // Test display
        let display = format!("{}", io_error);
        assert!(display.contains("IO error"));
        assert!(display.contains("test error"));
    }
    
    #[test]
    fn test_parse_error_source_chain() {
        let inner_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let parse_error = ParseError::Io(inner_error);
        
        // Test the error source chain
        let source = parse_error.source().unwrap();
        let downcast = source.downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(downcast.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(downcast.to_string(), "permission denied");
    }
    
    #[test]
    fn test_invalid_include_error_details_display() {
        // Test each variant of InvalidIncludeErrorDetails
        let pattern_error = InvalidIncludeErrorDetails::Pattern(
            glob::Pattern::new("[[").unwrap_err()
        );
        assert!(format!("{}", pattern_error).contains("Invalid glob pattern"));
        
        let io_error = InvalidIncludeErrorDetails::Io(
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied")
        );
        assert!(format!("{}", io_error).contains("IO error during include"));
        assert!(format!("{}", io_error).contains("access denied"));
        
        let hosts_error = InvalidIncludeErrorDetails::HostsInsideHostBlock;
        assert!(format!("{}", hosts_error).contains("Host definitions found inside host block"));
    }
}
