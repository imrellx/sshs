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
            InvalidIncludeErrorDetails::Pattern(e) => write!(f, "Invalid glob pattern: {e}"),
            InvalidIncludeErrorDetails::Glob(e) => write!(f, "Glob matching error: {e}"),
            InvalidIncludeErrorDetails::Io(e) => write!(f, "IO error during include: {e}"),
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
        write!(
            f,
            "Invalid include directive on line '{}': {}",
            self.line, self.details
        )
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
            ParseError::Io(e) => write!(f, "IO error: {e}"),
            ParseError::UnparseableLine(line) => write!(f, "Unable to parse line: '{line}'"),
            ParseError::UnknownEntry(e) => write!(f, "{e}"),
            ParseError::InvalidInclude(e) => write!(f, "{e}"),
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
