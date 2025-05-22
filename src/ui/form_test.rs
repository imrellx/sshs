use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;
use tui_input::Input;

use super::form::AddHostForm;

#[test]
fn test_add_host_form_validation() {
    let mut form = AddHostForm::new();
    
    // Initially the form should be invalid
    assert!(!form.is_valid());
    
    // Set only the host name - still invalid
    form.host_name = Input::from("test-host".to_string());
    assert!(!form.is_valid());
    
    // Set the hostname - now valid
    form.hostname = Input::from("test.example.com".to_string());
    assert!(form.is_valid());
    
    // Set optional fields - still valid
    form.username = Input::from("testuser".to_string());
    assert!(form.is_valid());
    
    form.port = Input::from("2222".to_string());
    assert!(form.is_valid());
}

#[test]
fn test_add_host_form_field_navigation() {
    let mut form = AddHostForm::new();
    
    // Initially the first field is active
    assert_eq!(form.active_field, 0);
    
    // Move to the next field
    form.next_field();
    assert_eq!(form.active_field, 1);
    
    // Move to the next field again
    form.next_field();
    assert_eq!(form.active_field, 2);
    
    // Move to the next field again
    form.next_field();
    assert_eq!(form.active_field, 3);
    
    // Move to the next field - should wrap around to the first field
    form.next_field();
    assert_eq!(form.active_field, 0);
    
    // Move to the previous field - should wrap around to the last field
    form.previous_field();
    assert_eq!(form.active_field, 3);
}

#[test]
fn test_add_host_to_config_file() {
    // Create a temporary file for testing
    let mut temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_owned();
    
    // Write initial content to the file
    writeln!(temp_file, "# SSH config file").unwrap();
    
    // Create a form with test data
    let mut form = AddHostForm::new();
    
    form.host_name = Input::from("TestHost".to_string());
    form.hostname = Input::from("test.example.com".to_string());
    form.username = Input::from("testuser".to_string());
    form.port = Input::from("2222".to_string());
    
    // Save the form to the config file
    let result = form.save_to_config(&temp_path);
    assert!(result.is_ok());
    
    // Read the content of the file
    let mut content = String::new();
    std::io::Read::read_to_string(&mut temp_file, &mut content).unwrap();
    
    // Check that the file contains the expected content
    assert!(content.contains("Host \"TestHost\""));
    assert!(content.contains("Hostname test.example.com"));
    assert!(content.contains("User testuser"));
    assert!(content.contains("Port 2222"));
    
    // Check that a backup file was created
    let backup_path = format!("{}.bak", temp_path);
    assert!(fs::metadata(&backup_path).is_ok());
    
    // Verify the backup file contains the original content
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, "# SSH config file\n");
    
    // Clean up
    fs::remove_file(backup_path).unwrap();
}

#[test]
fn test_missing_required_fields() {
    // Create a temporary file for testing
    let mut temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_owned();
    
    // Write initial content to the file
    writeln!(temp_file, "# SSH config file").unwrap();
    
    // Create a form without required fields
    let form = AddHostForm::new();
    
    // Saving should fail because required fields are missing
    let result = form.save_to_config(&temp_path);
    assert!(result.is_err());
    
    // Create a form with only host name
    let mut form = AddHostForm::new();
    form.host_name = Input::from("TestHost".to_string());
    
    // Saving should fail because hostname is missing
    let result = form.save_to_config(&temp_path);
    assert!(result.is_err());
    
    // Create a form with only hostname
    let mut form = AddHostForm::new();
    form.hostname = Input::from("test.example.com".to_string());
    
    // Saving should fail because host name is missing
    let result = form.save_to_config(&temp_path);
    assert!(result.is_err());
}