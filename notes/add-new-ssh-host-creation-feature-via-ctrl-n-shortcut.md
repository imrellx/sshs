# Add New SSH Host Creation Feature via Ctrl+N Shortcut

## Issue Summary
**Issue:** [#1 Add new SSH host creation feature via Ctrl+N shortcut](https://github.com/imrellx/sshs/issues/1)

**Problem:** Currently, users must manually edit the SSH config file outside of the sshs application to add new SSH connections, which breaks the seamless TUI experience.

**Solution:** Implement an "Add New Host" feature accessible via the Ctrl+N keyboard shortcut that allows users to create and save new SSH connections directly from the TUI.

## Requirements
1. Ctrl+N keyboard shortcut should open a new host creation dialog
2. Dialog should provide input fields for essential SSH connection parameters:
   - Host name (required)
   - Hostname/IP address (required)
   - Username (optional)
   - Port (optional, defaults to 22)
   - Additional SSH options (optional)
3. Form validation to ensure required fields are completed before saving
4. New host entries should be properly formatted and appended to the SSH config file
5. Dialog should be cancellable without saving changes
6. Success/error feedback should be provided to the user after save attempts
7. The new host should immediately appear in the main host list after successful creation

## User Experience
- Pressing Ctrl+N from the main TUI opens a modal dialog for adding a new SSH host
- Users can navigate between fields using Tab/Shift+Tab
- Enter submits the form when all required fields are valid
- Escape cancels the dialog and returns to the main interface without saving
- Required fields (Host name, Hostname) must be populated before allowing submission
- Invalid characters or formatting should be highlighted with error messages
- New entries should be appended to the primary SSH config file
- Success/error messages should be displayed after save operations

## Affected Components
- Main TUI interface (src/ui.rs)
- SSH config parsing and writing (src/ssh_config/)
- Host management (src/ssh.rs)

## Implementation Notes
- Follow existing TUI design patterns using ratatui components
- Consider adding a new "add host" dialog component
- Ensure proper error handling for file I/O operations
- Use the same color palette and styling as the main interface

## Implementation Plan (TDD Approach)

### 1. Design
- Define the new form dialog component structure
- Design the host creation form UI
- Plan the integration with existing TUI framework

### 2. Tests
- Write tests for the new form dialog component
- Test form validation logic
- Test SSH config file writing functionality
- Test host list refresh after adding a new host
- Test keyboard navigation and form submission

### 3. Implementation
- Create a new form dialog component
- Implement form validation logic
- Add Ctrl+N shortcut handler
- Implement SSH config file writing functionality
- Add success/error feedback messaging
- Refresh host list after adding a new host

### 4. Integration
- Integrate the new form dialog with the existing TUI
- Add keyboard navigation (Tab/Shift+Tab)
- Handle form submission and cancellation

### 5. Testing & Refinement
- Test the complete feature end-to-end
- Refine styling and UX based on testing
- Fix any bugs or edge cases

## Implementation Summary

The implementation was completed following the Test-Driven Development approach. Key parts of the implementation include:

1. **Code Structure Refactoring**: 
   - Refactored UI module into smaller, focused components
   - Created separate modules for app, form, and rendering

2. **Form Dialog Component**:
   - Implemented `AddHostForm` with fields for host details
   - Added validation logic for required fields
   - Implemented keyboard navigation between fields

3. **SSH Config Writing**:
   - Added functionality to write new host entries to SSH config file
   - Implemented backup creation before writing to the file
   - Added error handling for file operations

4. **UI Integration**:
   - Added Ctrl+N shortcut handler
   - Implemented form rendering in the TUI
   - Added feedback messaging for success/error states

5. **Testing**:
   - Added unit tests for form validation
   - Added tests for form navigation
   - Added tests for SSH config file writing

## Pull Request
To complete the PR, follow these steps:

1. Push the branch to your remote repository:
```
git push -u origin feature-add-host-dialog
```

2. Create a Pull Request through the GitHub web UI with the following details:

**Title**: Add new SSH host creation feature via Ctrl+N shortcut

**Body**:
```
## Summary
- Implements a new feature that allows users to add new SSH hosts directly from the TUI interface
- The Ctrl+N shortcut opens a form dialog for entering host details
- Form data is saved to the SSH config file with proper validation and error handling
- The host list is automatically refreshed after adding a new host

## Test plan
- Run the application and press Ctrl+N to open the new host form
- Test form navigation using Tab/Shift+Tab
- Try submitting the form with missing required fields (should show error)
- Try submitting with valid data and verify the host is added to the config file
- Verify the new host appears in the host list after being added

Closes #1
```