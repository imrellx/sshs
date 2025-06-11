use crate::ssh::Host;
use anyhow::Result;
use std::process::Child;
use std::time::Instant;

/// Maximum number of concurrent sessions (increased from MVP limit)
pub const MAX_SESSIONS: usize = 20;

/// Status of an SSH session
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    /// Session is connected and active
    Connected,
    /// Session is attempting to reconnect
    Reconnecting,
    /// Session is disconnected or failed
    Disconnected,
}

/// Activity indicators for session tabs
#[derive(Debug, Clone)]
pub struct ActivityIndicators {
    /// New output since last viewed (*)
    pub has_new_output: bool,
    /// Error or alert condition (!)
    pub has_error: bool,
    /// Background command running (@)
    pub has_background_activity: bool,
}

/// Represents a single SSH session tab
#[derive(Debug)]
pub struct Session {
    pub id: usize,
    pub host: Host,
    pub ssh_process: Option<Child>,
    pub is_active: bool,
    pub status: SessionStatus,
    pub activity: ActivityIndicators,
    pub last_activity: Option<Instant>,
}

impl Session {
    /// Create a new session
    #[must_use]
    pub fn new(id: usize, host: Host) -> Self {
        Self {
            id,
            host,
            ssh_process: None,
            is_active: false,
            status: SessionStatus::Connected,
            activity: ActivityIndicators {
                has_new_output: false,
                has_error: false,
                has_background_activity: false,
            },
            last_activity: None,
        }
    }

    /// Get the display name for the tab
    #[must_use]
    pub fn tab_display_name(&self) -> String {
        let mut indicators = String::new();

        // Add activity indicators
        if self.activity.has_new_output {
            indicators.push('*');
        }
        if self.activity.has_error {
            indicators.push('!');
        }
        if self.activity.has_background_activity {
            indicators.push('@');
        }

        if indicators.is_empty() {
            format!("[{}:{}]", self.id, self.host.name)
        } else {
            format!("[{}:{}{}]", self.id, self.host.name, indicators)
        }
    }

    /// Check if this session has an active SSH connection
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.ssh_process.is_some()
    }

    /// Mark session as having new output
    pub fn mark_new_output(&mut self) {
        self.activity.has_new_output = true;
        self.last_activity = Some(Instant::now());
    }

    /// Mark session as having an error
    pub fn mark_error(&mut self) {
        self.activity.has_error = true;
        self.status = SessionStatus::Disconnected;
    }

    /// Mark session as having background activity
    pub fn mark_background_activity(&mut self) {
        self.activity.has_background_activity = true;
        self.last_activity = Some(Instant::now());
    }

    /// Clear activity indicators (called when tab becomes active)
    pub fn clear_activity_indicators(&mut self) {
        self.activity.has_new_output = false;
        // Note: Keep error and background activity until manually cleared
    }

    /// Clear error indicator
    pub fn clear_error(&mut self) {
        self.activity.has_error = false;
        if !self.activity.has_new_output && !self.activity.has_background_activity {
            self.status = SessionStatus::Connected;
        }
    }
}

/// Manages multiple SSH sessions with tab functionality
#[derive(Debug)]
pub struct TabManager {
    sessions: Vec<Session>,
    current_session_index: usize,
    next_session_id: usize,
}

impl TabManager {
    /// Create a new tab manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            current_session_index: 0,
            next_session_id: 1,
        }
    }

    /// Add a new session if under the limit
    ///
    /// # Errors
    ///
    /// Will return `Err` if the maximum number of sessions is reached.
    pub fn add_session(&mut self, host: Host) -> Result<usize> {
        if self.sessions.len() >= MAX_SESSIONS {
            anyhow::bail!("Maximum number of sessions ({}) reached", MAX_SESSIONS);
        }

        let session_id = self.next_session_id;
        let session = Session::new(session_id, host);
        self.sessions.push(session);
        self.next_session_id += 1;

        // Switch to the new session
        self.current_session_index = self.sessions.len() - 1;

        Ok(session_id)
    }

    /// Switch to a session by 1-based index (for Ctrl+1, Ctrl+2, etc.)
    pub fn switch_to_session(&mut self, one_based_index: usize) -> bool {
        if one_based_index == 0 || one_based_index > self.sessions.len() {
            return false;
        }

        self.current_session_index = one_based_index - 1;
        true
    }

    /// Get the current active session
    #[must_use]
    pub fn current_session(&self) -> Option<&Session> {
        self.sessions.get(self.current_session_index)
    }

    /// Get all sessions for tab display
    #[must_use]
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// Get the current session index (0-based)
    #[must_use]
    pub fn current_session_index(&self) -> usize {
        self.current_session_index
    }

    /// Check if any sessions exist
    #[must_use]
    pub fn has_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }

    /// Get the number of active sessions
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Generate the tab bar display string
    #[must_use]
    pub fn tab_bar_display(&self) -> String {
        if self.sessions.is_empty() {
            return String::new();
        }

        self.sessions
            .iter()
            .enumerate()
            .map(|(index, session)| {
                let display = session.tab_display_name();
                if index == self.current_session_index {
                    format!("▶{display}") // Highlight current tab
                } else {
                    display
                }
            })
            .collect::<String>()
    }

    /// Mark activity on a specific session
    pub fn mark_session_activity(&mut self, session_index: usize, activity_type: &str) {
        if let Some(session) = self.sessions.get_mut(session_index) {
            match activity_type {
                "output" => session.mark_new_output(),
                "error" => session.mark_error(),
                "background" => session.mark_background_activity(),
                _ => {}
            }
        }
    }

    /// Switch to session and clear its activity indicators
    pub fn switch_to_session_and_clear_activity(&mut self, one_based_index: usize) -> bool {
        if self.switch_to_session(one_based_index) {
            if let Some(session) = self.sessions.get_mut(self.current_session_index) {
                session.clear_activity_indicators();
            }
            true
        } else {
            false
        }
    }

    /// Disconnect a session by index (0-based)
    pub fn disconnect_session(&mut self, session_index: usize) -> bool {
        if let Some(session) = self.sessions.get_mut(session_index) {
            // Set status to disconnected
            session.status = SessionStatus::Disconnected;

            // Terminate SSH process if running
            if let Some(mut process) = session.ssh_process.take() {
                let _ = process.kill(); // Ignore errors on kill
                let _ = process.wait(); // Clean up zombie process
            }

            // Mark as having an error to show disconnect indicator
            session.activity.has_error = true;

            true
        } else {
            false
        }
    }

    /// Rename a session by index (0-based)
    pub fn rename_session(&mut self, session_index: usize, new_name: String) -> bool {
        if let Some(session) = self.sessions.get_mut(session_index) {
            session.host.name = new_name;
            true
        } else {
            false
        }
    }

    /// Close (remove) a session by index (0-based)
    /// Returns true if a session was removed, false if index was invalid
    pub fn close_session(&mut self, session_index: usize) -> bool {
        if session_index >= self.sessions.len() {
            return false;
        }

        // Terminate SSH process if running before removing
        if let Some(session) = self.sessions.get_mut(session_index) {
            if let Some(mut process) = session.ssh_process.take() {
                let _ = process.kill();
                let _ = process.wait();
            }
        }

        // Remove the session
        self.sessions.remove(session_index);

        // Adjust current session index if necessary
        if self.sessions.is_empty() {
            // No sessions left
            self.current_session_index = 0;
        } else if self.current_session_index >= self.sessions.len() {
            // Current session was at/beyond the end, move to last session
            self.current_session_index = self.sessions.len() - 1;
        } else if self.current_session_index > session_index {
            // Current session was after the removed one, shift index down
            self.current_session_index -= 1;
        }
        // If current session was before the removed one, no change needed

        true
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_host(name: &str) -> Host {
        Host {
            name: name.to_string(),
            destination: format!("{name}.com"),
            user: Some("root".to_string()),
            port: Some("22".to_string()),
            aliases: String::new(),
            proxy_command: None,
        }
    }

    #[test]
    fn test_new_tab_manager_is_empty() {
        let manager = TabManager::new();
        assert!(!manager.has_sessions());
        assert_eq!(manager.session_count(), 0);
        assert!(manager.current_session().is_none());
    }

    #[test]
    fn test_add_first_session() {
        let mut manager = TabManager::new();
        let host = create_test_host("prod-web");

        let session_id = manager.add_session(host).unwrap();

        assert_eq!(session_id, 1);
        assert!(manager.has_sessions());
        assert_eq!(manager.session_count(), 1);
        assert_eq!(manager.current_session_index(), 0);

        let current = manager.current_session().unwrap();
        assert_eq!(current.id, 1);
        assert_eq!(current.host.name, "prod-web");
    }

    #[test]
    fn test_add_multiple_sessions() {
        let mut manager = TabManager::new();

        let _id1 = manager.add_session(create_test_host("host1")).unwrap();
        let _id2 = manager.add_session(create_test_host("host2")).unwrap();
        let id3 = manager.add_session(create_test_host("host3")).unwrap();

        assert_eq!(manager.session_count(), 3);
        assert_eq!(id3, 3);
        assert_eq!(manager.current_session_index(), 2); // Should be on the last added

        let current = manager.current_session().unwrap();
        assert_eq!(current.host.name, "host3");
    }

    #[test]
    fn test_maximum_sessions_limit() {
        let mut manager = TabManager::new();

        // Add maximum sessions
        for i in 1..=MAX_SESSIONS {
            let host = create_test_host(&format!("host{i}"));
            manager.add_session(host).unwrap();
        }

        assert_eq!(manager.session_count(), MAX_SESSIONS);

        // Try to add one more - should fail
        let result = manager.add_session(create_test_host("extra"));
        assert!(result.is_err());
        assert_eq!(manager.session_count(), MAX_SESSIONS);
    }

    #[test]
    fn test_switch_to_session_valid_indices() {
        let mut manager = TabManager::new();

        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();
        manager.add_session(create_test_host("host3")).unwrap();

        // Should start on session 3 (last added)
        assert_eq!(manager.current_session_index(), 2);

        // Switch to session 1 (Ctrl+1)
        assert!(manager.switch_to_session(1));
        assert_eq!(manager.current_session_index(), 0);
        assert_eq!(manager.current_session().unwrap().host.name, "host1");

        // Switch to session 2 (Ctrl+2)
        assert!(manager.switch_to_session(2));
        assert_eq!(manager.current_session_index(), 1);
        assert_eq!(manager.current_session().unwrap().host.name, "host2");
    }

    #[test]
    fn test_switch_to_session_invalid_indices() {
        let mut manager = TabManager::new();
        manager.add_session(create_test_host("host1")).unwrap();

        // Test invalid indices
        assert!(!manager.switch_to_session(0)); // 0 is invalid (1-based)
        assert!(!manager.switch_to_session(2)); // Only 1 session exists
        assert!(!manager.switch_to_session(99)); // Way out of range

        // Should still be on the original session
        assert_eq!(manager.current_session_index(), 0);
    }

    #[test]
    fn test_session_tab_display_name() {
        let host = create_test_host("prod-web");
        let session = Session::new(1, host);

        assert_eq!(session.tab_display_name(), "[1:prod-web]");
    }

    #[test]
    fn test_tab_bar_display_single_session() {
        let mut manager = TabManager::new();
        manager.add_session(create_test_host("prod-web")).unwrap();

        let display = manager.tab_bar_display();
        assert_eq!(display, "▶[1:prod-web]");
    }

    #[test]
    fn test_tab_bar_display_multiple_sessions() {
        let mut manager = TabManager::new();
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();
        manager.add_session(create_test_host("host3")).unwrap();

        // Should highlight the last session (current)
        let display = manager.tab_bar_display();
        assert_eq!(display, "[1:host1][2:host2]▶[3:host3]");

        // Switch to first session and check display
        manager.switch_to_session(1);
        let display = manager.tab_bar_display();
        assert_eq!(display, "▶[1:host1][2:host2][3:host3]");
    }

    #[test]
    fn test_tab_bar_display_empty() {
        let manager = TabManager::new();
        assert_eq!(manager.tab_bar_display(), "");
    }

    #[test]
    fn test_create_many_sessions() {
        let mut manager = TabManager::new();

        // Should be able to create 10 sessions (more than current 3 limit)
        for i in 1..=10 {
            let host = create_test_host(&format!("host{i}"));
            let session_id = manager.add_session(host).unwrap();
            assert_eq!(session_id, i);
        }

        assert_eq!(manager.session_count(), 10);
        assert_eq!(manager.current_session_index(), 9); // Last added session

        // Verify we can navigate through all sessions
        for i in 1..=10 {
            assert!(manager.switch_to_session(i));
            assert_eq!(manager.current_session_index(), i - 1); // 0-based index
            let current = manager.current_session().unwrap();
            assert_eq!(current.host.name, format!("host{i}"));
        }
    }

    #[test]
    fn test_reasonable_session_limit() {
        let mut manager = TabManager::new();

        // Should be able to create up to 20 sessions
        for i in 1..=20 {
            let host = create_test_host(&format!("host{i}"));
            manager.add_session(host).unwrap();
        }

        assert_eq!(manager.session_count(), 20);

        // Try to add one more - should fail at reasonable limit
        let result = manager.add_session(create_test_host("extra"));
        assert!(result.is_err());
        assert_eq!(manager.session_count(), 20);
    }

    #[test]
    fn test_tab_bar_display_overflow() {
        let mut manager = TabManager::new();

        // Create many sessions to test overflow display
        for i in 1..=15 {
            let host = create_test_host(&format!("host{i}"));
            manager.add_session(host).unwrap();
        }

        // Test that tab bar display includes all sessions
        let display = manager.tab_bar_display();

        // Should contain all 15 tabs
        for i in 1..=15 {
            assert!(
                display.contains(&format!("[{i}:host{i}]")),
                "Display should contain tab {i}: {display}"
            );
        }

        // Should highlight the last session (current)
        assert!(
            display.contains("▶[15:host15]"),
            "Should highlight current tab: {display}"
        );
    }

    #[test]
    fn test_session_is_connected_initially_false() {
        let host = create_test_host("test");
        let session = Session::new(1, host);
        assert!(!session.is_connected());
    }

    #[test]
    fn test_activity_indicators_basic() {
        let host = create_test_host("test");
        let mut session = Session::new(1, host);

        // Initially no indicators
        assert_eq!(session.tab_display_name(), "[1:test]");
        assert_eq!(session.status, SessionStatus::Connected);

        // Mark new output
        session.mark_new_output();
        assert_eq!(session.tab_display_name(), "[1:test*]");
        assert!(session.activity.has_new_output);

        // Mark error
        session.mark_error();
        assert_eq!(session.tab_display_name(), "[1:test*!]");
        assert!(session.activity.has_error);
        assert_eq!(session.status, SessionStatus::Disconnected);

        // Mark background activity
        session.mark_background_activity();
        assert_eq!(session.tab_display_name(), "[1:test*!@]");
        assert!(session.activity.has_background_activity);
    }

    #[test]
    fn test_clear_activity_indicators() {
        let host = create_test_host("test");
        let mut session = Session::new(1, host);

        // Set all activity indicators
        session.mark_new_output();
        session.mark_error();
        session.mark_background_activity();
        assert_eq!(session.tab_display_name(), "[1:test*!@]");

        // Clear activity indicators (only clears new output)
        session.clear_activity_indicators();
        assert_eq!(session.tab_display_name(), "[1:test!@]");
        assert!(!session.activity.has_new_output);
        assert!(session.activity.has_error);
        assert!(session.activity.has_background_activity);

        // Clear error manually
        session.clear_error();
        assert_eq!(session.tab_display_name(), "[1:test@]");
        assert!(!session.activity.has_error);
    }

    #[test]
    fn test_tab_manager_activity_marking() {
        let mut manager = TabManager::new();

        // Add two sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();

        // Mark activity on first session
        manager.mark_session_activity(0, "output");
        let display = manager.tab_bar_display();
        assert!(display.contains("[1:host1*]"));
        assert!(display.contains("[2:host2]"));

        // Mark error on second session
        manager.mark_session_activity(1, "error");
        let display = manager.tab_bar_display();
        assert!(display.contains("[1:host1*]"));
        assert!(display.contains("▶[2:host2!]")); // Second is current
    }

    #[test]
    fn test_switch_and_clear_activity() {
        let mut manager = TabManager::new();

        // Add sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();

        // Mark activity on first session and switch to it
        manager.mark_session_activity(0, "output");
        assert!(manager.switch_to_session_and_clear_activity(1));

        // Activity should be cleared on first session (now current)
        let display = manager.tab_bar_display();
        assert!(display.contains("▶[1:host1]")); // No * indicator
        assert!(display.contains("[2:host2]"));
    }

    #[test]
    fn test_disconnect_session() {
        let mut manager = TabManager::new();

        // Add sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();

        // Initially both sessions should be connected
        assert_eq!(manager.sessions()[0].status, SessionStatus::Connected);
        assert_eq!(manager.sessions()[1].status, SessionStatus::Connected);
        assert!(!manager.sessions()[0].activity.has_error);
        assert!(!manager.sessions()[1].activity.has_error);

        // Disconnect first session
        assert!(manager.disconnect_session(0));

        // First session should be disconnected with error indicator
        assert_eq!(manager.sessions()[0].status, SessionStatus::Disconnected);
        assert!(manager.sessions()[0].activity.has_error);

        // Second session should remain connected
        assert_eq!(manager.sessions()[1].status, SessionStatus::Connected);
        assert!(!manager.sessions()[1].activity.has_error);
    }

    #[test]
    fn test_disconnect_session_invalid_index() {
        let mut manager = TabManager::new();

        // Try to disconnect non-existent session
        assert!(!manager.disconnect_session(0));
        assert!(!manager.disconnect_session(99));

        // Add one session and try to disconnect invalid index
        manager.add_session(create_test_host("host1")).unwrap();
        assert!(!manager.disconnect_session(1)); // Only index 0 exists
        assert!(manager.disconnect_session(0)); // Valid index should work
    }

    #[test]
    fn test_rename_session() {
        let mut manager = TabManager::new();

        // Add sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();

        // Initially sessions have original names
        assert_eq!(manager.sessions()[0].host.name, "host1");
        assert_eq!(manager.sessions()[1].host.name, "host2");

        // Rename first session
        assert!(manager.rename_session(0, "new-host1".to_string()));
        assert_eq!(manager.sessions()[0].host.name, "new-host1");
        assert_eq!(manager.sessions()[1].host.name, "host2"); // Second unchanged

        // Rename second session
        assert!(manager.rename_session(1, "new-host2".to_string()));
        assert_eq!(manager.sessions()[0].host.name, "new-host1");
        assert_eq!(manager.sessions()[1].host.name, "new-host2");
    }

    #[test]
    fn test_rename_session_invalid_index() {
        let mut manager = TabManager::new();

        // Try to rename non-existent session
        assert!(!manager.rename_session(0, "new-name".to_string()));
        assert!(!manager.rename_session(99, "new-name".to_string()));

        // Add one session and try to rename invalid index
        manager.add_session(create_test_host("host1")).unwrap();
        assert!(!manager.rename_session(1, "new-name".to_string())); // Only index 0 exists
        assert!(manager.rename_session(0, "new-name".to_string())); // Valid index should work
        assert_eq!(manager.sessions()[0].host.name, "new-name");
    }

    #[test]
    fn test_close_session_middle() {
        let mut manager = TabManager::new();

        // Add three sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();
        manager.add_session(create_test_host("host3")).unwrap();

        // Current session should be index 2 (last added)
        assert_eq!(manager.current_session_index(), 2);
        assert_eq!(manager.session_count(), 3);

        // Close middle session (index 1)
        assert!(manager.close_session(1));

        // Should have 2 sessions left
        assert_eq!(manager.session_count(), 2);
        assert_eq!(manager.sessions()[0].host.name, "host1");
        assert_eq!(manager.sessions()[1].host.name, "host3");

        // Current session index should shift down since it was after the removed session
        assert_eq!(manager.current_session_index(), 1);
    }

    #[test]
    fn test_close_session_current() {
        let mut manager = TabManager::new();

        // Add two sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();

        // Current session is index 1 (last added)
        assert_eq!(manager.current_session_index(), 1);

        // Close current session (index 1)
        assert!(manager.close_session(1));

        // Should have 1 session left
        assert_eq!(manager.session_count(), 1);
        assert_eq!(manager.sessions()[0].host.name, "host1");

        // Current session should move to index 0 (last remaining)
        assert_eq!(manager.current_session_index(), 0);
    }

    #[test]
    fn test_close_session_before_current() {
        let mut manager = TabManager::new();

        // Add three sessions
        manager.add_session(create_test_host("host1")).unwrap();
        manager.add_session(create_test_host("host2")).unwrap();
        manager.add_session(create_test_host("host3")).unwrap();

        // Switch to middle session (index 1)
        manager.switch_to_session(2); // 1-based, so index 1
        assert_eq!(manager.current_session_index(), 1);

        // Close first session (index 0, before current)
        assert!(manager.close_session(0));

        // Should have 2 sessions left
        assert_eq!(manager.session_count(), 2);
        assert_eq!(manager.sessions()[0].host.name, "host2");
        assert_eq!(manager.sessions()[1].host.name, "host3");

        // Current session should shift down from 1 to 0
        assert_eq!(manager.current_session_index(), 0);
    }

    #[test]
    fn test_close_last_remaining_session() {
        let mut manager = TabManager::new();

        // Add one session
        manager.add_session(create_test_host("only-host")).unwrap();
        assert_eq!(manager.session_count(), 1);
        assert_eq!(manager.current_session_index(), 0);

        // Close the only session
        assert!(manager.close_session(0));

        // Should have no sessions left
        assert_eq!(manager.session_count(), 0);
        assert!(!manager.has_sessions());
        assert_eq!(manager.current_session_index(), 0); // Reset to 0
    }

    #[test]
    fn test_close_session_invalid_index() {
        let mut manager = TabManager::new();

        // Try to close non-existent session
        assert!(!manager.close_session(0));
        assert!(!manager.close_session(99));

        // Add one session
        manager.add_session(create_test_host("host1")).unwrap();

        // Try to close invalid index
        assert!(!manager.close_session(1));
        assert!(!manager.close_session(99));

        // Valid index should work
        assert!(manager.close_session(0));
        assert_eq!(manager.session_count(), 0);
    }
}
