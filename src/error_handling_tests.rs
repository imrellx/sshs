use anyhow::Result;
use crate::ssh::ParseConfigError;
use crate::ssh_config::parser_error::ParseError;

// Tests to ensure all errors implement proper traits and have consistent behavior
#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use std::error::Error;
    
    #[test]
    fn test_all_errors_implement_error_trait() {
        // Test ParseConfigError
        let parse_config_error = ParseConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test"
        ));
        let _: &dyn Error = &parse_config_error;
        
        // Test ParseError
        let parse_error = ParseError::UnparseableLine("invalid line".to_string());
        let _: &dyn Error = &parse_error;
        
        // All errors should be displayable
        assert!(!format!("{}", parse_config_error).is_empty());
        assert!(!format!("{}", parse_error).is_empty());
    }
    
    #[test]
    fn test_error_source_chain() {
        // Test that error chains are properly implemented
        let inner_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let parse_config_error = ParseConfigError::Io(inner_error);
        
        // Should have a source
        assert!(parse_config_error.source().is_some());
        
        // The source should be the original IO error
        let source = parse_config_error.source().unwrap();
        let io_error = source.downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
    }
    
    #[test]
    fn test_error_messages_are_helpful() {
        // Test that error messages contain useful information
        let parse_config_error = ParseConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "permission denied"
        ));
        
        let message = format!("{}", parse_config_error);
        assert!(message.contains("IO error"));
        assert!(message.contains("permission denied"));
        
        let parse_error = ParseError::UnparseableLine("invalid line content".to_string());
        let message = format!("{}", parse_error);
        assert!(message.contains("Unable to parse line"));
        assert!(message.contains("invalid line content"));
    }
    
    #[test]
    fn test_error_conversion() {
        // Test that error conversion works properly
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let parse_error: ParseError = io_error.into();
        
        match parse_error {
            ParseError::Io(ref e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
                assert_eq!(e.to_string(), "not found");
            }
            _ => panic!("Expected IO error variant"),
        }
    }
    
    #[test]
    fn test_anyhow_integration() {
        // Test that our errors work well with anyhow
        fn might_fail() -> Result<()> {
            Err(ParseConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "access denied"
            )))?
        }
        
        let result = might_fail();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = format!("{}", error);
        assert!(error_string.contains("IO error"));
        assert!(error_string.contains("access denied"));
    }
    
    #[test]
    fn test_error_context() {
        // Test that errors can be wrapped with additional context
        use anyhow::Context;
        
        fn parse_with_context(path: &str) -> Result<()> {
            Err(ParseConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found"
            )))
            .with_context(|| format!("Failed to parse SSH config at {}", path))?
        }
        
        let result = parse_with_context("/home/user/.ssh/config");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error); // Use Debug format to see context
        assert!(error_string.contains("Failed to parse SSH config at /home/user/.ssh/config"));
    }
}
