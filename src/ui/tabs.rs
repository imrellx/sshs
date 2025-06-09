use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use ratatui::{
    prelude::*,
    widgets::*,
    style::palette::tailwind,
};

use crate::ssh::Host;
use super::session::{SshSession, SessionConfig, ConnectionState, ActivityIndicator};

/// Maximum number of tabs that can be displayed at once
const MAX_VISIBLE_TABS: usize = 10;

/// Tab overflow handling
#[derive(Debug, Clone)]
pub struct TabOverflow {
    /// Starting index for visible tabs
    pub start_index: usize,
    /// Whether there are tabs to the left
    pub has_left_overflow: bool,
    /// Whether there are tabs to the right
    pub has_right_overflow: bool,
}

/// Manages multiple SSH sessions in a tab-based interface
pub struct TabManager {
    /// All active sessions
    sessions: Vec<SshSession>,
    
    /// Index of the currently active tab
    active_tab_index: usize,
    
    /// Session configuration
    session_config: SessionConfig,
    
    /// Tab overflow state for handling many tabs
    overflow: TabOverflow,
    
    /// Color palette for rendering
    palette: tailwind::Palette,
    
    /// Last update time for activity monitoring
    last_update: Instant,
    
    /// Map of session IDs to their tab indices for quick lookup
    session_id_map: HashMap<String, usize>,
}

impl TabManager {
    /// Create a new tab manager
    pub fn new(session_config: SessionConfig, palette: tailwind::Palette) -> Self {
        Self {
            sessions: Vec::new(),
            active_tab_index: 0,
            session_config,
            overflow: TabOverflow {
                start_index: 0,
                has_left_overflow: false,
                has_right_overflow: false,
            },
            palette,
            last_update: Instant::now(),
            session_id_map: HashMap::new(),
        }
    }

    /// Create a new SSH session in a new tab
    pub fn create_session(&mut self, host: Host) -> Result<usize> {
        let mut session = SshSession::new(host, self.session_config.clone());
        
        // Attempt to connect
        if let Err(e) = session.connect() {
            // Still add the session even if connection fails
            eprintln!("Failed to connect to {}: {}", session.host.name, e);
        }

        let session_id = session.id.clone();
        let tab_index = self.sessions.len();
        
        self.sessions.push(session);
        self.session_id_map.insert(session_id, tab_index);
        
        // Switch to the new tab
        self.active_tab_index = tab_index;
        self.update_overflow();
        
        Ok(tab_index)
    }

    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize) -> Result<()> {
        if index >= self.sessions.len() {
            return Err(anyhow::anyhow!("Invalid tab index: {}", index));
        }

        // Remove session and clean up
        let session = self.sessions.remove(index);
        self.session_id_map.remove(&session.id);
        
        // Update session ID map for shifted indices
        for (_session_id, tab_index) in self.session_id_map.iter_mut() {
            if *tab_index > index {
                *tab_index -= 1;
            }
        }

        // Adjust active tab index
        if self.sessions.is_empty() {
            self.active_tab_index = 0;
        } else if self.active_tab_index >= self.sessions.len() {
            self.active_tab_index = self.sessions.len() - 1;
        } else if self.active_tab_index > index {
            self.active_tab_index -= 1;
        }

        self.update_overflow();
        Ok(())
    }

    /// Close the currently active tab
    pub fn close_current_tab(&mut self) -> Result<()> {
        if self.sessions.is_empty() {
            return Err(anyhow::anyhow!("No tabs to close"));
        }
        self.close_tab(self.active_tab_index)
    }

    /// Switch to the next tab
    pub fn next_tab(&mut self) {
        if !self.sessions.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.sessions.len();
            self.update_overflow();
            self.mark_current_tab_viewed();
        }
    }

    /// Switch to the previous tab
    pub fn previous_tab(&mut self) {
        if !self.sessions.is_empty() {
            self.active_tab_index = if self.active_tab_index == 0 {
                self.sessions.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.update_overflow();
            self.mark_current_tab_viewed();
        }
    }

    /// Switch to a specific tab by number (1-based)
    pub fn goto_tab(&mut self, tab_number: usize) -> Result<()> {
        if tab_number == 0 || tab_number > self.sessions.len() {
            return Err(anyhow::anyhow!("Invalid tab number: {}", tab_number));
        }
        
        self.active_tab_index = tab_number - 1;
        self.update_overflow();
        self.mark_current_tab_viewed();
        Ok(())
    }

    /// Get the currently active session
    pub fn get_active_session(&self) -> Option<&SshSession> {
        self.sessions.get(self.active_tab_index)
    }

    /// Get the currently active session mutably
    pub fn get_active_session_mut(&mut self) -> Option<&mut SshSession> {
        self.sessions.get_mut(self.active_tab_index)
    }

    /// Get a session by index
    pub fn get_session(&self, index: usize) -> Option<&SshSession> {
        self.sessions.get(index)
    }

    /// Get a session by index mutably
    pub fn get_session_mut(&mut self, index: usize) -> Option<&mut SshSession> {
        self.sessions.get_mut(index)
    }

    /// Get all sessions
    pub fn get_all_sessions(&self) -> &[SshSession] {
        &self.sessions
    }

    /// Get the number of sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Check if there are any sessions
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Get the active tab index
    pub fn get_active_tab_index(&self) -> usize {
        self.active_tab_index
    }

    /// Process pending data for all sessions
    pub fn process_all_sessions(&mut self) {
        for session in &mut self.sessions {
            session.process_pending_data();
            session.update_state();
        }
    }

    /// Mark the current tab as viewed
    pub fn mark_current_tab_viewed(&mut self) {
        if let Some(session) = self.get_active_session_mut() {
            session.mark_viewed();
        }
    }

    /// Update tab overflow state
    fn update_overflow(&mut self) {
        let total_tabs = self.sessions.len();
        
        if total_tabs <= MAX_VISIBLE_TABS {
            // All tabs fit
            self.overflow = TabOverflow {
                start_index: 0,
                has_left_overflow: false,
                has_right_overflow: false,
            };
        } else {
            // Tabs overflow - center the active tab when possible
            let half_visible = MAX_VISIBLE_TABS / 2;
            
            let ideal_start = if self.active_tab_index >= half_visible {
                self.active_tab_index - half_visible
            } else {
                0
            };
            
            // Ensure we don't show past the end
            let max_start = total_tabs.saturating_sub(MAX_VISIBLE_TABS);
            let start_index = ideal_start.min(max_start);
            
            self.overflow = TabOverflow {
                start_index,
                has_left_overflow: start_index > 0,
                has_right_overflow: start_index + MAX_VISIBLE_TABS < total_tabs,
            };
        }
    }

    /// Get the range of visible tabs
    pub fn get_visible_tab_range(&self) -> (usize, usize) {
        let start = self.overflow.start_index;
        let end = (start + MAX_VISIBLE_TABS).min(self.sessions.len());
        (start, end)
    }

    /// Check if a tab index is currently visible
    pub fn is_tab_visible(&self, index: usize) -> bool {
        let (start, end) = self.get_visible_tab_range();
        index >= start && index < end
    }

    /// Render the tab bar
    pub fn render_tab_bar(&self, _area: Rect) -> Tabs {
        let (start, end) = self.get_visible_tab_range();
        let mut tab_titles = Vec::new();
        
        // Add left overflow indicator
        if self.overflow.has_left_overflow {
            tab_titles.push(Line::from("..."));
        }
        
        // Add visible tabs
        for (i, session) in self.sessions[start..end].iter().enumerate() {
            let actual_index = start + i;
            let tab_number = actual_index + 1;
            let (display_name, activity, state) = session.get_tab_info();
            
            // Build tab title with indicators
            let mut spans = vec![Span::styled(
                format!("{}:", tab_number),
                Style::default().fg(self.palette.c300)
            )];
            
            // Add activity indicator
            let indicator_span = match activity {
                ActivityIndicator::NewOutput => Span::styled("*", Style::default().fg(Color::Yellow)),
                ActivityIndicator::Error => Span::styled("!", Style::default().fg(Color::Red)),
                ActivityIndicator::BackgroundActivity => Span::styled("@", Style::default().fg(Color::Blue)),
                ActivityIndicator::None => Span::raw(""),
            };
            
            if !indicator_span.content.is_empty() {
                spans.push(indicator_span);
            }
            
            // Add display name with color based on connection state
            let name_color = match state {
                ConnectionState::Connected => Color::Green,
                ConnectionState::Connecting => Color::Yellow,
                ConnectionState::Reconnecting => Color::Yellow,
                ConnectionState::Disconnected => self.palette.c400,
                ConnectionState::Error(_) => Color::Red,
            };
            
            spans.push(Span::styled(
                display_name,
                Style::default().fg(name_color)
            ));
            
            tab_titles.push(Line::from(spans));
        }
        
        // Add right overflow indicator
        if self.overflow.has_right_overflow {
            tab_titles.push(Line::from("..."));
        }
        
        // Add "+" for new tab
        if !self.overflow.has_right_overflow || end == self.sessions.len() {
            tab_titles.push(Line::from(Span::styled(
                "+",
                Style::default().fg(self.palette.c300)
            )));
        }

        // Calculate selected index for visible range
        let selected = if self.active_tab_index >= start && self.active_tab_index < end {
            let offset = if self.overflow.has_left_overflow { 1 } else { 0 };
            Some(self.active_tab_index - start + offset)
        } else {
            None
        };

        Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(self.palette.c300))
                    .title(" SSH Sessions ")
                    .title_style(Style::default().fg(self.palette.c500).add_modifier(Modifier::BOLD))
            )
            .style(Style::default().fg(self.palette.c200))
            .highlight_style(
                Style::default()
                    .fg(self.palette.c100)
                    .bg(self.palette.c600)
                    .add_modifier(Modifier::BOLD)
            )
            .select(selected.unwrap_or(0))
    }

    /// Handle terminal resize for all sessions
    pub fn resize_all_sessions(&mut self, cols: u16, rows: u16) -> Result<()> {
        for session in &mut self.sessions {
            session.resize(cols, rows)?;
        }
        Ok(())
    }

    /// Send input to the active session
    pub fn send_input_to_active(&self, input: &[u8]) -> Result<()> {
        if let Some(session) = self.get_active_session() {
            session.send_input(input)?;
        }
        Ok(())
    }

    /// Send command to the active session
    pub fn send_command_to_active(&self, command: &str) -> Result<()> {
        if let Some(session) = self.get_active_session() {
            session.send_command(command)?;
        }
        Ok(())
    }

    /// Get session statistics for session manager overlay
    pub fn get_session_stats(&self) -> Vec<SessionStats> {
        self.sessions
            .iter()
            .enumerate()
            .map(|(i, session)| SessionStats {
                tab_number: i + 1,
                name: session.display_name.clone(),
                host: session.host.destination.clone(),
                state: session.state.clone(),
                activity: session.activity.clone(),
                stats: session.get_stats(),
                is_active: i == self.active_tab_index,
            })
            .collect()
    }

    /// Attempt to reconnect a session by index
    pub fn reconnect_session(&mut self, index: usize) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(index) {
            session.reconnect()?;
        }
        Ok(())
    }

    /// Attempt to reconnect the active session
    pub fn reconnect_active_session(&mut self) -> Result<()> {
        let index = self.active_tab_index;
        self.reconnect_session(index)
    }

    /// Clean up disconnected sessions (optional background task)
    pub fn cleanup_disconnected_sessions(&mut self) {
        // Remove sessions that have been disconnected for too long
        let cleanup_threshold = Duration::from_secs(300); // 5 minutes
        let now = Instant::now();
        
        let mut indices_to_remove = Vec::new();
        
        for (i, session) in self.sessions.iter().enumerate() {
            if matches!(session.state, ConnectionState::Disconnected) {
                // TODO: Add proper last_activity access when field is made public
                // For now, just check disconnected state
                indices_to_remove.push(i);
            }
        }
        
        // Remove from highest index to lowest to avoid index shifting issues
        for &index in indices_to_remove.iter().rev() {
            let _ = self.close_tab(index);
        }
    }

    /// Find session by host name
    pub fn find_session_by_host(&self, hostname: &str) -> Option<usize> {
        self.sessions
            .iter()
            .position(|session| session.host.name == hostname)
    }

    /// Check if any session has the given host
    pub fn has_session_for_host(&self, hostname: &str) -> bool {
        self.find_session_by_host(hostname).is_some()
    }
}

/// Statistics for session manager overlay
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub tab_number: usize,
    pub name: String,
    pub host: String,
    pub state: ConnectionState,
    pub activity: ActivityIndicator,
    pub stats: super::session::SessionStats,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::Host;

    fn create_test_host(name: &str) -> Host {
        Host {
            name: name.to_string(),
            destination: "localhost".to_string(),
            user: Some("testuser".to_string()),
            port: Some("22".to_string()),
            aliases: "".to_string(),
            proxy_command: None,
        }
    }

    #[tokio::test]
    async fn test_tab_manager_creation() {
        let config = SessionConfig::default();
        let palette = tailwind::BLUE;
        let manager = TabManager::new(config, palette);
        
        assert_eq!(manager.session_count(), 0);
        assert!(manager.is_empty());
        assert_eq!(manager.get_active_tab_index(), 0);
    }

    #[tokio::test]
    async fn test_create_session() {
        let config = SessionConfig::default();
        let palette = tailwind::BLUE;
        let mut manager = TabManager::new(config, palette);
        
        let host = create_test_host("test1");
        let result = manager.create_session(host).await;
        
        assert!(result.is_ok());
        assert_eq!(manager.session_count(), 1);
        assert_eq!(manager.get_active_tab_index(), 0);
    }

    #[tokio::test]
    async fn test_tab_navigation() {
        let config = SessionConfig::default();
        let palette = tailwind::BLUE;
        let mut manager = TabManager::new(config, palette);
        
        // Create multiple sessions
        for i in 1..=3 {
            let host = create_test_host(&format!("test{}", i));
            manager.create_session(host).await.unwrap();
        }
        
        assert_eq!(manager.session_count(), 3);
        assert_eq!(manager.get_active_tab_index(), 2); // Last created
        
        // Test navigation
        manager.next_tab();
        assert_eq!(manager.get_active_tab_index(), 0); // Wraps around
        
        manager.previous_tab();
        assert_eq!(manager.get_active_tab_index(), 2); // Wraps around
        
        // Test goto
        manager.goto_tab(2).unwrap();
        assert_eq!(manager.get_active_tab_index(), 1); // 1-based to 0-based
    }

    #[tokio::test]
    async fn test_close_tab() {
        let config = SessionConfig::default();
        let palette = tailwind::BLUE;
        let mut manager = TabManager::new(config, palette);
        
        // Create sessions
        for i in 1..=3 {
            let host = create_test_host(&format!("test{}", i));
            manager.create_session(host).await.unwrap();
        }
        
        assert_eq!(manager.session_count(), 3);
        
        // Close middle tab
        manager.close_tab(1).unwrap();
        assert_eq!(manager.session_count(), 2);
        
        // Close current tab
        manager.close_current_tab().unwrap();
        assert_eq!(manager.session_count(), 1);
        
        // Close last tab
        manager.close_current_tab().unwrap();
        assert_eq!(manager.session_count(), 0);
        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn test_overflow_handling() {
        let config = SessionConfig::default();
        let palette = tailwind::BLUE;
        let mut manager = TabManager::new(config, palette);
        
        // Create more tabs than MAX_VISIBLE_TABS
        for i in 1..=15 {
            let host = create_test_host(&format!("test{}", i));
            manager.create_session(host).await.unwrap();
        }
        
        let (start, end) = manager.get_visible_tab_range();
        assert!(end - start <= MAX_VISIBLE_TABS);
        assert!(manager.overflow.has_right_overflow);
    }

    #[test]
    fn test_find_session_by_host() {
        let config = SessionConfig::default();
        let palette = tailwind::BLUE;
        let mut manager = TabManager::new(config, palette);
        
        // Add some sessions manually for testing
        let host1 = create_test_host("web-server");
        let host2 = create_test_host("db-server");
        
        let session1 = SshSession::new(host1, SessionConfig::default());
        let session2 = SshSession::new(host2, SessionConfig::default());
        
        manager.sessions.push(session1);
        manager.sessions.push(session2);
        
        assert_eq!(manager.find_session_by_host("web-server"), Some(0));
        assert_eq!(manager.find_session_by_host("db-server"), Some(1));
        assert_eq!(manager.find_session_by_host("nonexistent"), None);
        
        assert!(manager.has_session_for_host("web-server"));
        assert!(!manager.has_session_for_host("nonexistent"));
    }
}