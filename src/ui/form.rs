use anyhow::{Result, anyhow};
use crossterm::event::Event;
use std::fs::{self, OpenOptions};
use std::io::Write;
use tui_input::{backend::crossterm::EventHandler, Input};

/// Represents the state of the form dialog
#[derive(PartialEq, Copy, Clone)]
pub enum FormState {
    /// Form is hidden
    Hidden,
    /// Form is active and visible
    Active,
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
        match self.active_field {
            0 => { self.host_name.handle_event(event); }
            1 => { self.hostname.handle_event(event); }
            2 => { self.username.handle_event(event); }
            3 => { self.port.handle_event(event); }
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

    /// Check if the form is valid (required fields are filled)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.host_name.value().trim().is_empty() && !self.hostname.value().trim().is_empty()
    }

    /// Get the current active input
    #[must_use]
    pub fn active_input(&self) -> &Input {
        match self.active_field {
            0 => &self.host_name,
            1 => &self.hostname,
            2 => &self.username,
            3 => &self.port,
            _ => &self.host_name,
        }
    }

    /// Get the current active input mutably
    pub fn active_input_mut(&mut self) -> &mut Input {
        match self.active_field {
            0 => &mut self.host_name,
            1 => &mut self.hostname,
            2 => &mut self.username,
            3 => &mut self.port,
            _ => &mut self.host_name,
        }
    }

    /// Save the form data to the SSH config file
    /// 
    /// # Errors
    /// 
    /// Will return `Err` if the file cannot be opened or written to
    pub fn save_to_config(&self, config_path: &str) -> Result<()> {
        // First, validate if the form data is valid
        if !self.is_valid() {
            return Err(anyhow!("Required fields are not filled"));
        }

        // Prepare the SSH config entry
        let mut entry = format!("\nHost \"{}\"\n", self.host_name.value().trim());
        entry.push_str(&format!("  Hostname {}\n", self.hostname.value().trim()));
        
        if !self.username.value().trim().is_empty() {
            entry.push_str(&format!("  User {}\n", self.username.value().trim()));
        }
        
        if !self.port.value().trim().is_empty() {
            entry.push_str(&format!("  Port {}\n", self.port.value().trim()));
        }

        // Check if the file exists
        if !std::path::Path::new(config_path).exists() {
            return Err(anyhow!("SSH config file does not exist"));
        }

        // Create a backup of the original config file
        let backup_path = format!("{}.bak", config_path);
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
        let backup_path = format!("{}.bak", temp_path);
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
}