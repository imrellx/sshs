use anyhow::{Result, anyhow};
use crossterm::event::Event;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use tui_input::{backend::crossterm::EventHandler, Input};
use crate::ssh;

/// Represents the state of the form dialog
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum FormState {
    /// Form is hidden
    Hidden,
    /// Form is active and visible
    Active,
    /// Showing confirmation dialog
    Confirming,
}

/// Form for adding a new SSH host
pub struct AddHostForm {
    /// Host name (pattern)
    pub host_name: Input,
    /// Hostname or IP address
    pub hostname: Input,
    /// Username (optional)
    pub username: Input,
    /// Port (optional, defaults to 22)
    pub port: Input,
    /// Current active field index
    pub active_field: usize,
    /// Total number of fields
    pub field_count: usize,
}

impl Default for AddHostForm {
    fn default() -> Self {
        Self::new()
    }
}

impl AddHostForm {
    /// Create a new add host form with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_name: Input::default(),
            hostname: Input::default(),
            username: Input::default(),
            port: Input::default(),
            active_field: 0,
            field_count: 4,
        }
    }

    /// Handle input events for the form
    pub fn handle_event(&mut self, event: &Event) {
        // Special handling for port field to ensure numeric input only
        if self.active_field == 3 {
            if let Event::Key(key) = event {
                if let crossterm::event::KeyCode::Char(c) = key.code {
                    // Only handle numeric characters for port field
                    if c.is_ascii_digit() || key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        self.port.handle_event(event);
                    }
                    // Skip non-numeric characters
                    return;
                }
                // Allow navigation keys and other special keys
                self.port.handle_event(event);
            }
            return;
        }
        
        // Normal handling for other fields
        match self.active_field {
            0 => { self.host_name.handle_event(event); }
            1 => { self.hostname.handle_event(event); }
            2 => { self.username.handle_event(event); }
            _ => { /* Do nothing */ }
        }
    }

    /// Move to the next field
    pub fn next_field(&mut self) {
        self.active_field = (self.active_field + 1) % self.field_count;
    }

    /// Move to the previous field
    pub fn previous_field(&mut self) {
        self.active_field = if self.active_field == 0 {
            self.field_count - 1
        } else {
            self.active_field - 1
        };
    }

    /// Check if the form is valid (required fields are filled and values are valid)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        // Check required fields
        let has_required_fields = !self.host_name.value().trim().is_empty() && 
                                 !self.hostname.value().trim().is_empty();
        
        // Check hostname format is valid
        let hostname_valid = self.is_valid_hostname();
        
        // Check username format is valid
        let username_valid = self.is_valid_username();
        
        // Check port is valid if provided
        let port_valid = if self.port.value().trim().is_empty() {
            true // Empty port is valid (will use default SSH port)
        } else {
            self.port.value().trim().parse::<u16>().is_ok()
        };
        
        has_required_fields && hostname_valid && username_valid && port_valid
    }
    
    /// Validate hostname format (IP address or domain name)
    fn is_valid_hostname(&self) -> bool {
        let hostname = self.hostname.value().trim();
        if hostname.is_empty() {
            return false;
        }
        
        // Simple validation - ensure hostname doesn't contain invalid characters
        // More complex validation could check for valid domain name or IP format
        !hostname.contains(|c: char| c.is_whitespace() || c == '?' || c == '*' || c == '#')
    }
    
    /// Validate username format
    fn is_valid_username(&self) -> bool {
        let username = self.username.value().trim();
        if username.is_empty() {
            return true; // Empty username is valid (optional field)
        }
        
        // Simple validation - ensure username doesn't contain invalid characters
        !username.contains(|c: char| c.is_whitespace() || c == '/' || c == ':' || c == '\\')
    }

    /// Get validation error message if form is not valid
    #[must_use]
    pub fn validation_error(&self) -> Option<String> {
        // Check required fields
        if self.host_name.value().trim().is_empty() || self.hostname.value().trim().is_empty() {
            return Some("Please fill out required fields".to_string());
        }
        
        // Validate hostname format
        if !self.is_valid_hostname() {
            return Some("Invalid hostname format".to_string());
        }
        
        // Validate username format
        if !self.is_valid_username() {
            return Some("Invalid username format".to_string());
        }
        
        // Validate port number
        if !self.port.value().trim().is_empty() && self.port.value().trim().parse::<u16>().is_err() {
            return Some("Port must be a valid number (0-65535)".to_string());
        }
        
        None
    }

    /// Get the current active input
    #[must_use]
    pub fn active_input(&self) -> &Input {
        match self.active_field {
            1 => &self.hostname,
            2 => &self.username,
            3 => &self.port,
            _ => &self.host_name,
        }
    }

    /// Get the current active input mutably
    pub fn active_input_mut(&mut self) -> &mut Input {
        match self.active_field {
            1 => &mut self.hostname,
            2 => &mut self.username,
            3 => &mut self.port,
            _ => &mut self.host_name,
        }
    }

    /// Sanitize input to prevent security issues and ensure valid SSH config
    fn sanitize_host_name(&self) -> String {
        // Trim whitespace and escape any special characters in Host pattern
        let host_name = self.host_name.value().trim();
        
        // If the host name doesn't have quotes already, wrap it in quotes to handle spaces
        // If it already has quotes, use it as is (trimmed)
        if !host_name.starts_with('"') && !host_name.ends_with('"') && host_name.contains(' ') {
            format!("\"{host_name}\"")
        } else {
            host_name.to_string()
        }
    }
    
    /// Sanitize hostname/IP value
    fn sanitize_hostname(&self) -> String {
        // Trim whitespace and remove any potentially problematic characters
        self.hostname.value().trim().to_string()
    }
    
    /// Sanitize username value
    fn sanitize_username(&self) -> String {
        // Trim whitespace and remove any potentially problematic characters
        self.username.value().trim().to_string()
    }
    
    /// Sanitize port value and return as string
    fn sanitize_port(&self) -> Option<String> {
        let port = self.port.value().trim();
        if port.is_empty() {
            None
        } else {
            // This is already validated to be a valid number
            Some(port.to_string())
        }
    }
    
    /// Check if a host with the same name already exists in the SSH config file
    /// 
    /// # Errors
    /// 
    /// Will return `Err` if the file cannot be read
    fn host_exists(&self, config_path: &str, host_name: &str) -> Result<bool> {
        let file = File::open(config_path)
            .map_err(|e| anyhow!("Failed to open SSH config file: {}", e))?;
        
        let reader = BufReader::new(file);
        
        // Remove quotes if they exist for comparison
        let clean_host_name = host_name.trim_matches('"');
        
        // Simplified pattern matching for Host entries
        for line in reader.lines() {
            let line = line.map_err(|e| anyhow!("Failed to read line from SSH config file: {}", e))?;
            let trimmed = line.trim();
            
            // Look for lines that start with "Host"
            if trimmed.starts_with("Host ") {
                // Extract the host pattern (everything after "Host ")
                let pattern = trimmed["Host ".len()..].trim();
                
                // Remove quotes for comparison if they exist
                let clean_pattern = pattern.trim_matches('"');
                
                // Check if the pattern matches our host name
                if clean_pattern == clean_host_name {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }

    /// Check if a host with the same name would be a duplicate
    /// 
    /// # Errors
    /// 
    /// Will return `Err` if the file cannot be read
    pub fn check_duplicate(&self, config_path: &str) -> Result<bool> {
        if !self.is_valid() {
            return Ok(false); // Invalid form can't be a duplicate
        }
        
        let host_name = self.sanitize_host_name();
        let host_exists = self.host_exists(config_path, &host_name)?;
        
        Ok(host_exists)
    }

    /// Save the form data to the SSH config file
    /// 
    /// # Errors
    /// 
    /// Will return `Err` if the file cannot be opened or written to
    pub fn save_to_config(&self, config_path: &str) -> Result<()> {
        // First, validate if the form data is valid
        if !self.is_valid() {
            return Err(anyhow!("Form validation failed"));
        }

        // Sanitize inputs and prepare the SSH config entry
        let host_name = self.sanitize_host_name();
        let destination = self.sanitize_hostname();
        let username = self.sanitize_username();
        let port = self.sanitize_port();
        
        // Build the SSH config entry
        let mut entry = format!("\nHost {host_name}\n");
        entry.push_str(&format!("  Hostname {destination}\n"));
        
        if let Some(username) = (!username.is_empty()).then_some(username) {
            entry.push_str(&format!("  User {username}\n"));
        }
        
        if let Some(port) = port {
            entry.push_str(&format!("  Port {port}\n"));
        }

        // Check if the file exists
        if !std::path::Path::new(config_path).exists() {
            return Err(anyhow!("SSH config file does not exist"));
        }
        
        // Note: We no longer need to check for duplicates here, since the app handles it before calling this method

        // Create a backup of the original config file
        let backup_path = format!("{config_path}.bak");
        fs::copy(config_path, &backup_path)
            .map_err(|e| anyhow!("Failed to create backup of SSH config file: {}", e))?;

        // Open the file in append mode
        let mut file = OpenOptions::new()
            .append(true)
            .open(config_path)
            .map_err(|e| anyhow!("Failed to open SSH config file: {}", e))?;

        // Write the entry to the file
        file.write_all(entry.as_bytes())
            .map_err(|e| anyhow!("Failed to write to SSH config file: {}", e))?;

        Ok(())
    }
    
    /// Populate the form with data from an existing SSH host
    pub fn populate_from_host(&mut self, host: &ssh::Host) {
        self.host_name = Input::from(host.name.clone());
        self.hostname = Input::from(host.destination.clone());
        
        if let Some(user) = &host.user {
            self.username = Input::from(user.clone());
        }
        
        if let Some(port) = &host.port {
            self.port = Input::from(port.clone());
        }
    }
    
    /// Update an existing host entry in the SSH config file
    /// 
    /// # Errors
    /// 
    /// Will return `Err` if the file cannot be read or written to
    pub fn update_host_in_config(&self, config_path: &str, original_host: &ssh::Host) -> Result<()> {
        // First, validate if the form data is valid
        if !self.is_valid() {
            return Err(anyhow!("Form validation failed"));
        }

        // Read the entire config file
        let content = fs::read_to_string(config_path)
            .map_err(|e| anyhow!("Failed to read SSH config file: {}", e))?;

        // Create a backup of the original config file
        let backup_path = format!("{config_path}.bak");
        fs::copy(config_path, &backup_path)
            .map_err(|e| anyhow!("Failed to create backup of SSH config file: {}", e))?;

        // Find and replace the host entry
        let updated_content = self.replace_host_entry(&content, original_host)?;

        // Write the updated content back to the file
        fs::write(config_path, updated_content)
            .map_err(|e| anyhow!("Failed to write updated SSH config file: {}", e))?;

        Ok(())
    }
    
    /// Replace a host entry in the SSH config content
    fn replace_host_entry(&self, content: &str, original_host: &ssh::Host) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;
        
        // Find the host entry to replace
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for Host lines that match our original host name
            if line.starts_with("Host ") {
                let pattern = line["Host ".len()..].trim();
                let clean_pattern = pattern.trim_matches('"');
                
                if clean_pattern == original_host.name {
                    // Found the host entry to replace
                    // Skip this host block and add our new one
                    i += 1;
                    
                    // Skip all lines until the next Host block or end of file
                    while i < lines.len() {
                        let next_line = lines[i].trim();
                        if next_line.starts_with("Host ") && !next_line.is_empty() {
                            break;
                        }
                        i += 1;
                    }
                    
                    // Add the new host entry
                    let new_entry = self.build_host_entry();
                    result.push(new_entry);
                    
                    continue;
                }
            }
            
            result.push(lines[i].to_string());
            i += 1;
        }
        
        Ok(result.join("\n"))
    }
    
    /// Build a complete host entry string
    fn build_host_entry(&self) -> String {
        let host_name = self.sanitize_host_name();
        let destination = self.sanitize_hostname();
        let username = self.sanitize_username();
        let port = self.sanitize_port();
        
        let mut entry = format!("Host {host_name}");
        entry.push_str(&format!("\n  Hostname {destination}"));
        
        if !username.is_empty() {
            entry.push_str(&format!("\n  User {username}"));
        }
        
        if let Some(port) = port {
            entry.push_str(&format!("\n  Port {port}"));
        }
        
        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_form_validation() {
        let mut form = AddHostForm::new();
        assert!(!form.is_valid());

        // Insert text into host_name field
        form.host_name = Input::from("Test Host".to_string());
        assert!(!form.is_valid());

        // Insert text into hostname field
        form.hostname = Input::from("localhost".to_string());
        assert!(form.is_valid());

        // Test with valid port
        form.port = Input::from("22".to_string());
        assert!(form.is_valid());

        // Test with invalid port (non-numeric)
        form.port = Input::from("abc".to_string());
        assert!(!form.is_valid());
        assert_eq!(
            form.validation_error(),
            Some("Port must be a valid number (0-65535)".to_string())
        );

        // Test with invalid port (out of range)
        form.port = Input::from("99999".to_string());
        assert!(!form.is_valid());
        assert_eq!(
            form.validation_error(),
            Some("Port must be a valid number (0-65535)".to_string())
        );

        // Test with valid port (upper range)
        form.port = Input::from("65535".to_string());
        assert!(form.is_valid());
        
        // Test invalid hostname format
        form.hostname = Input::from("invalid hostname?".to_string());
        assert!(!form.is_valid());
        assert_eq!(
            form.validation_error(),
            Some("Invalid hostname format".to_string())
        );
        
        // Test invalid username format
        form.hostname = Input::from("valid-hostname".to_string());
        form.username = Input::from("invalid/username".to_string());
        assert!(!form.is_valid());
        assert_eq!(
            form.validation_error(),
            Some("Invalid username format".to_string())
        );
        
        // Reset to valid values
        form.hostname = Input::from("example.com".to_string());
        form.username = Input::from("validuser".to_string());
        assert!(form.is_valid());
    }
    
    #[test]
    fn test_sanitize_functions() {
        let mut form = AddHostForm::new();
        
        // Test host name sanitization
        form.host_name = Input::from("  Test Host  ".to_string());
        assert_eq!(form.sanitize_host_name(), "\"Test Host\"");
        
        form.host_name = Input::from("NoSpaces".to_string());
        assert_eq!(form.sanitize_host_name(), "NoSpaces");
        
        // Test hostname sanitization
        form.hostname = Input::from("  example.com  ".to_string());
        assert_eq!(form.sanitize_hostname(), "example.com");
        
        // Test username sanitization
        form.username = Input::from("  user  ".to_string());
        assert_eq!(form.sanitize_username(), "user");
        
        // Test port sanitization
        form.port = Input::from("  22  ".to_string());
        assert_eq!(form.sanitize_port(), Some("22".to_string()));
        
        form.port = Input::from(String::new());
        assert_eq!(form.sanitize_port(), None);
    }

    #[test]
    fn test_form_navigation() {
        let mut form = AddHostForm::new();
        assert_eq!(form.active_field, 0);

        form.next_field();
        assert_eq!(form.active_field, 1);

        form.next_field();
        assert_eq!(form.active_field, 2);

        form.previous_field();
        assert_eq!(form.active_field, 1);

        form.previous_field();
        assert_eq!(form.active_field, 0);

        form.previous_field();
        assert_eq!(form.active_field, 3);
    }

    #[test]
    fn test_save_to_config() -> Result<()> {
        // Create a temporary file for testing
        let mut temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap().to_owned();

        // Write some initial content
        writeln!(temp_file, "# SSH Config File")?;

        // Create a form with test data
        let mut form = AddHostForm::new();
        
        form.host_name = Input::from("Test Host".to_string());
        form.hostname = Input::from("test.example.com".to_string());
        form.username = Input::from("testuser".to_string());
        form.port = Input::from("2222".to_string());

        // Save the form to the config file
        form.save_to_config(&temp_path)?;

        // Read the file content
        let mut content = String::new();
        temp_file.read_to_string(&mut content)?;

        // Check if the content contains the expected entry
        assert!(content.contains("Host \"Test Host\""));
        assert!(content.contains("Hostname test.example.com"));
        assert!(content.contains("User testuser"));
        assert!(content.contains("Port 2222"));

        // Verify backup file was created
        let backup_path = format!("{temp_path}.bak");
        assert!(std::path::Path::new(&backup_path).exists());

        // Clean up
        fs::remove_file(backup_path)?;

        Ok(())
    }

    #[test]
    fn test_save_to_config_missing_required_fields() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap().to_owned();

        // Create a form with missing required fields
        let form = AddHostForm::new();

        // Save should fail due to missing required fields
        let result = form.save_to_config(&temp_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_populate_from_host() {
        use crate::ssh::Host;
        
        let mut form = AddHostForm::new();
        let host = Host {
            name: "test-host".to_string(),
            destination: "test.example.com".to_string(),
            user: Some("testuser".to_string()),
            port: Some("2222".to_string()),
            aliases: String::new(),
            proxy_command: None,
        };

        form.populate_from_host(&host);
        
        assert_eq!(form.host_name.value(), "test-host");
        assert_eq!(form.hostname.value(), "test.example.com");
        assert_eq!(form.username.value(), "testuser");
        assert_eq!(form.port.value(), "2222");
    }

    #[test]
    fn test_update_host_in_config() -> Result<()> {
        use crate::ssh::Host;
        
        // Create a temporary file with existing SSH config
        let mut temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap().to_owned();

        // Write initial config with a host to edit
        writeln!(temp_file, "# SSH Config File")?;
        writeln!(temp_file, "Host old-host")?;
        writeln!(temp_file, "  Hostname old.example.com")?;
        writeln!(temp_file, "  User olduser")?;
        writeln!(temp_file, "  Port 22")?;
        writeln!(temp_file)?;
        writeln!(temp_file, "Host another-host")?;
        writeln!(temp_file, "  Hostname another.example.com")?;

        // Create the original host object
        let original_host = Host {
            name: "old-host".to_string(),
            destination: "old.example.com".to_string(),
            user: Some("olduser".to_string()),
            port: Some("22".to_string()),
            aliases: String::new(),
            proxy_command: None,
        };

        // Create a form with updated data
        let mut form = AddHostForm::new();
        form.host_name = Input::from("updated-host".to_string());
        form.hostname = Input::from("updated.example.com".to_string());
        form.username = Input::from("updateduser".to_string());
        form.port = Input::from("2222".to_string());

        // Update the host in the config file
        form.update_host_in_config(&temp_path, &original_host)?;

        // Read the updated file content
        let content = fs::read_to_string(&temp_path)?;

        // Check if the host was updated correctly
        assert!(content.contains("Host updated-host"));
        assert!(content.contains("Hostname updated.example.com"));
        assert!(content.contains("User updateduser"));
        assert!(content.contains("Port 2222"));
        
        // Check that other hosts are preserved
        assert!(content.contains("Host another-host"));
        assert!(content.contains("Hostname another.example.com"));
        
        // Check that old host data is removed
        assert!(!content.contains("Host old-host"));
        assert!(!content.contains("old.example.com"));

        // Verify backup file was created
        let backup_path = format!("{temp_path}.bak");
        assert!(std::path::Path::new(&backup_path).exists());

        // Clean up
        fs::remove_file(backup_path)?;

        Ok(())
    }
}