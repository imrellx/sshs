#[cfg(test)]
mod security_tests {
    use crate::ssh::Host;

    #[test]
    fn test_allowlist_validation() {
        // Test characters that should be allowed in a safe command
        let safe_chars = [
            'a', 'z', 'A', 'Z', '0', '9', ' ', '.', '-', '_', '@', ':', '/',
            '%', '+', '=', ',',
        ];

        for c in safe_chars {
            let host = Host {
                name: format!("test{c}host"),
                aliases: "".to_string(),
                user: Some("user".to_string()),
                destination: "example.com".to_string(),
                port: Some("22".to_string()),
                proxy_command: None,
            };

            // Use a non-executing template for validation testing
            let template = "echo {{name}}";
            let result = host.run_command_template(template);
            assert!(result.is_err() == false, "Safe character '{}' was incorrectly rejected", c);
        }
    }

    #[test]
    fn test_comprehensive_dangerous_chars() {
        // Test a more comprehensive list of dangerous characters beyond
        // what's currently checked in validate_safe_for_command
        let dangerous_chars = [
            '&', '|', ';', '`', '$', '(', ')', '{', '}', '<', '>', '\n', '\r',
            // Additional dangerous characters
            '\\', '\'', '"', '!', '#', '*', '?', '[', ']', '^', '~'
        ];

        for c in dangerous_chars {
            let host = Host {
                name: format!("test{}host", c),
                aliases: "".to_string(),
                user: Some("user".to_string()),
                destination: "example.com".to_string(),
                port: Some("22".to_string()),
                proxy_command: None,
            };

            // Use a non-executing template for validation testing
            let template = "echo {{name}}";
            let result = host.run_command_template(template);
            assert!(result.is_err(), "Dangerous character '{}' was not detected", c);
        }
    }

    #[test]
    fn test_command_context_validation() {
        // Test that dangerous characters are detected in all relevant fields
        // not just the name field
        let dangerous_fields = [
            ("user", Some("user; ls".to_string()), "example.com".to_string(), None),
            ("destination", None, "example.com | cat /etc/passwd".to_string(), None),
            ("port", None, "example.com".to_string(), Some("22; rm -rf /".to_string())),
        ];

        for (field_name, user, destination, port) in dangerous_fields {
            let host = Host {
                name: "test".to_string(),
                aliases: "".to_string(),
                user,
                destination,
                port,
                proxy_command: None,
            };

            // Use a template that references the field being tested
            let template = match field_name {
                "user" => "echo {{user}}",
                "destination" => "echo {{destination}}",
                "port" => "echo {{port}}",
                _ => panic!("Unknown field name in test"),
            };

            let result = host.run_command_template(template);
            assert!(result.is_err(), "Dangerous character in {} was not detected", field_name);
        }
    }

    #[test]
    fn test_real_world_injection_scenarios() {
        // Test realistic injection patterns that might be used in an attack
        let attack_patterns = [
            "legitimate; cat /etc/passwd",
            "legitimate && cat /etc/passwd",
            "legitimate || cat /etc/passwd",
            "legitimate | cat /etc/passwd",
            "legitimate `cat /etc/passwd`",
            "legitimate$(cat /etc/passwd)",
            "legitimate${PATH}",
            "legitimate\ncat /etc/passwd",
            "legitimate/**/cat /etc/passwd",
            r#"legitimate\"; cat /etc/passwd"#,
        ];

        for pattern in attack_patterns {
            let host = Host {
                name: pattern.to_string(),
                aliases: "".to_string(),
                user: None,
                destination: "example.com".to_string(),
                port: None,
                proxy_command: None,
            };

            let template = "echo {{name}}";
            let result = host.run_command_template(template);
            assert!(result.is_err(), "Attack pattern '{}' was not detected", pattern);
        }
    }

    #[test]
    fn test_tilde_character_blocked() {
        // Test that tilde is properly blocked
        let host = Host {
            name: "test~host".to_string(),
            aliases: "".to_string(),
            user: Some("user".to_string()),
            destination: "example.com".to_string(),
            port: Some("22".to_string()),
            proxy_command: None,
        };

        // Use a non-executing template for validation testing
        let template = "echo {{name}}";
        let result = host.run_command_template(template);
        assert!(result.is_err(), "Tilde character was not detected as dangerous");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("dangerous characters"), "Error message should mention dangerous characters");
    }
} 