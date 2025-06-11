use crate::ssh::Host;
use anyhow::Result;
use std::process::Child;

/// Maximum number of concurrent sessions for MVP
pub const MAX_SESSIONS: usize = 3;

/// Represents a single SSH session tab
#[derive(Debug)]
pub struct Session {
    pub id: usize,
    pub host: Host,
    pub ssh_process: Option<Child>,
    pub is_active: bool,
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
        }
    }

    /// Get the display name for the tab
    #[must_use]
    pub fn tab_display_name(&self) -> String {
        format!("[{}:{}]", self.id, self.host.name)
    }

    /// Check if this session has an active SSH connection
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.ssh_process.is_some()
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
    fn test_session_is_connected_initially_false() {
        let host = create_test_host("test");
        let session = Session::new(1, host);
        assert!(!session.is_connected());
    }
}
