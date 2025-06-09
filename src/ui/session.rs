use anyhow::Result;
use portable_pty::{CommandBuilder, PtySize, PtySystem, Child, MasterPty};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread;
use vt100::Parser;

use crate::ssh::Host;

/// Represents the connection state of an SSH session
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Session is connecting
    Connecting,
    /// Session is connected and active
    Connected,
    /// Session is disconnected
    Disconnected,
    /// Session encountered an error
    Error(String),
    /// Session is attempting to reconnect
    Reconnecting,
}

/// Activity indicator for tabs
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityIndicator {
    /// No special activity
    None,
    /// New output since last viewed
    NewOutput,
    /// Connection error or disconnected
    Error,
    /// Background command running
    BackgroundActivity,
}

/// Configuration for SSH sessions
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum number of lines to keep in scrollback buffer
    pub max_scrollback_lines: usize,
    /// Timeout for connection attempts
    pub connection_timeout: Duration,
    /// Whether to attempt automatic reconnection
    pub auto_reconnect: bool,
    /// Number of reconnection attempts
    pub max_reconnect_attempts: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_scrollback_lines: 10_000,
            connection_timeout: Duration::from_secs(30),
            auto_reconnect: true,
            max_reconnect_attempts: 3,
        }
    }
}

/// Manages terminal buffer with scrollback and VT100 parsing
pub struct TerminalBuffer {
    parser: Parser,
    scrollback: VecDeque<String>,
    max_lines: usize,
    has_new_output: bool,
    last_viewed: Instant,
}

impl TerminalBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            parser: Parser::new(80, 24, 0), // Default size, will be updated
            scrollback: VecDeque::new(),
            max_lines,
            has_new_output: false,
            last_viewed: Instant::now(),
        }
    }

    /// Process raw data from PTY and update the terminal buffer
    pub fn process_data(&mut self, data: &[u8]) {
        self.parser.process(data);
        self.has_new_output = true;
        
        // Extract new lines from the parser screen
        let screen = self.parser.screen();
        let contents = screen.contents();
        
        // Add new content to scrollback
        for line in contents.lines() {
            self.scrollback.push_back(line.to_string());
            
            // Trim scrollback if it exceeds maximum
            if self.scrollback.len() > self.max_lines {
                self.scrollback.pop_front();
            }
        }
    }

    /// Resize the terminal
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.parser.set_size(rows, cols);
    }

    /// Get the current screen contents
    pub fn get_screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Get scrollback buffer
    pub fn get_scrollback(&self) -> &VecDeque<String> {
        &self.scrollback
    }

    /// Check if there's new output since last viewed
    pub fn has_new_output(&self) -> bool {
        self.has_new_output
    }

    /// Mark output as viewed
    pub fn mark_viewed(&mut self) {
        self.has_new_output = false;
        self.last_viewed = Instant::now();
    }

    /// Search within the terminal buffer
    pub fn search(&self, query: &str) -> Vec<(usize, String)> {
        let mut results = Vec::new();
        
        for (i, line) in self.scrollback.iter().enumerate() {
            if line.to_lowercase().contains(&query.to_lowercase()) {
                results.push((i, line.clone()));
            }
        }
        
        results
    }
}

/// Represents an individual SSH session with terminal emulation
pub struct SshSession {
    /// Host configuration
    pub host: Host,
    
    /// Session ID for identification
    pub id: String,
    
    /// Display name for the tab
    pub display_name: String,
    
    /// Current connection state
    pub state: ConnectionState,
    
    /// Activity indicator
    pub activity: ActivityIndicator,
    
    /// Terminal buffer with VT100 parsing
    pub terminal: TerminalBuffer,
    
    /// PTY master for the SSH process
    pty_master: Option<Box<dyn MasterPty + Send>>,
    
    /// SSH child process
    child_process: Option<Box<dyn Child + Send + Sync>>,
    
    /// Session configuration
    config: SessionConfig,
    
    /// Connection attempts counter
    reconnect_attempts: u32,
    
    /// Last activity timestamp
    last_activity: Instant,
    
    /// Channel for receiving data from PTY
    data_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    
    /// Channel for sending commands to session
    command_sender: Option<mpsc::Sender<String>>,
}

impl SshSession {
    /// Create a new SSH session
    pub fn new(host: Host, config: SessionConfig) -> Self {
        let id = format!("{}_{}", host.name, Instant::now().elapsed().as_millis());
        let display_name = Self::generate_display_name(&host);
        
        Self {
            host,
            id,
            display_name,
            state: ConnectionState::Disconnected,
            activity: ActivityIndicator::None,
            terminal: TerminalBuffer::new(config.max_scrollback_lines),
            pty_master: None,
            child_process: None,
            config,
            reconnect_attempts: 0,
            last_activity: Instant::now(),
            data_receiver: None,
            command_sender: None,
        }
    }

    /// Generate a display name for the tab from host configuration
    fn generate_display_name(host: &Host) -> String {
        // Try to create a concise but descriptive name
        let name = &host.name;
        
        // Truncate if too long
        if name.len() > 15 {
            format!("{}...", &name[..12])
        } else {
            name.clone()
        }
    }

    /// Start the SSH connection
    pub fn connect(&mut self) -> Result<()> {
        self.state = ConnectionState::Connecting;
        self.activity = ActivityIndicator::None;
        
        // Create PTY system
        let pty_system = portable_pty::native_pty_system();
        
        // Set up PTY size (will be updated by UI)
        let pty_size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Create PTY pair
        let pty_pair = pty_system.openpty(pty_size)?;
        
        // Build SSH command
        let mut cmd = CommandBuilder::new("ssh");
        
        // Add common SSH options
        cmd.arg("-o");
        cmd.arg("LogLevel=ERROR");
        cmd.arg("-o");
        cmd.arg("StrictHostKeyChecking=accept-new");
        
        // Add port if specified
        if let Some(port) = &self.host.port {
            cmd.arg("-p");
            cmd.arg(port);
        }
        
        // Add user and host
        let user = self.host.user.as_deref().unwrap_or("root");
        let connection_string = format!("{}@{}", user, self.host.destination);
        cmd.arg(connection_string);

        // Spawn the SSH process
        match pty_pair.slave.spawn_command(cmd) {
            Ok(child) => {
                self.child_process = Some(child);
                self.pty_master = Some(pty_pair.master);
                self.state = ConnectionState::Connected;
                self.last_activity = Instant::now();
                
                // Set up data channels
                self.setup_data_channels()?;
                
                Ok(())
            }
            Err(e) => {
                self.state = ConnectionState::Error(format!("Failed to start SSH: {}", e));
                self.activity = ActivityIndicator::Error;
                Err(anyhow::anyhow!("Failed to start SSH process: {}", e))
            }
        }
    }

    /// Set up channels for reading PTY data
    fn setup_data_channels(&mut self) -> Result<()> {
        if let Some(_pty_master) = &mut self.pty_master {
            let (data_tx, data_rx) = mpsc::channel();
            let (cmd_tx, _cmd_rx) = mpsc::channel();
            
            self.data_receiver = Some(data_rx);
            self.command_sender = Some(cmd_tx);
            
            // TODO: Implement proper PTY data handling
            // For now, just set up the channels without background threads
            // This will be implemented in a future iteration
            
            // Placeholder: simulate some data
            let _ = data_tx.send(b"SSH session connected...\n".to_vec());
        }
        
        Ok(())
    }

    /// Process any pending data from the PTY
    pub fn process_pending_data(&mut self) {
        if let Some(receiver) = &mut self.data_receiver {
            while let Ok(data) = receiver.try_recv() {
                self.terminal.process_data(&data);
                self.last_activity = Instant::now();
                self.activity = ActivityIndicator::NewOutput;
            }
        }
    }

    /// Send a command to the SSH session
    pub fn send_command(&self, command: &str) -> Result<()> {
        if let Some(sender) = &self.command_sender {
            sender.send(command.to_string())
                .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        }
        Ok(())
    }

    /// Send raw input (like key presses) to the session
    pub fn send_input(&self, _input: &[u8]) -> Result<()> {
        // TODO: Implement proper input forwarding to PTY
        // For now, this is a placeholder
        Ok(())
    }

    /// Resize the terminal
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        // Update terminal buffer
        self.terminal.resize(cols, rows);
        
        // Update PTY size
        if let Some(pty_master) = &self.pty_master {
            let pty_size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };
            pty_master.resize(pty_size)?;
        }
        
        Ok(())
    }

    /// Check if the session is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, ConnectionState::Connected | ConnectionState::Connecting)
    }

    /// Check if the session process is still running
    pub fn is_process_running(&self) -> bool {
        // TODO: Implement proper process checking
        // For now, just return true if we have a process
        self.child_process.is_some()
    }

    /// Update connection state based on process status
    pub fn update_state(&mut self) {
        if !self.is_process_running() && self.is_active() {
            self.state = ConnectionState::Disconnected;
            self.activity = ActivityIndicator::Error;
        }
    }

    /// Attempt to reconnect the session
    pub fn reconnect(&mut self) -> Result<()> {
        if self.reconnect_attempts >= self.config.max_reconnect_attempts {
            self.state = ConnectionState::Error("Max reconnection attempts exceeded".to_string());
            return Err(anyhow::anyhow!("Max reconnection attempts exceeded"));
        }

        self.reconnect_attempts += 1;
        self.state = ConnectionState::Reconnecting;
        
        // Clean up existing connection
        self.disconnect();
        
        // Wait a bit before reconnecting
        thread::sleep(Duration::from_secs(2));
        
        // Attempt to reconnect
        self.connect()
    }

    /// Disconnect the session
    pub fn disconnect(&mut self) {
        // Kill the child process
        if let Some(child) = &mut self.child_process {
            let _ = child.kill();
        }
        
        // Clean up resources
        self.child_process = None;
        self.pty_master = None;
        self.data_receiver = None;
        self.command_sender = None;
        
        self.state = ConnectionState::Disconnected;
    }

    /// Get display information for the tab
    pub fn get_tab_info(&self) -> (String, ActivityIndicator, ConnectionState) {
        (self.display_name.clone(), self.activity.clone(), self.state.clone())
    }

    /// Mark this session as viewed (clears new output indicator)
    pub fn mark_viewed(&mut self) {
        self.terminal.mark_viewed();
        if self.activity == ActivityIndicator::NewOutput {
            self.activity = ActivityIndicator::None;
        }
    }

    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        SessionStats {
            uptime: self.last_activity.elapsed(),
            lines_in_buffer: self.terminal.scrollback.len(),
            connection_attempts: self.reconnect_attempts,
            last_activity: self.last_activity,
        }
    }
}

/// Statistics about a session
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub uptime: Duration,
    pub lines_in_buffer: usize,
    pub connection_attempts: u32,
    pub last_activity: Instant,
}

impl Drop for SshSession {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_host() -> Host {
        Host {
            name: "test-host".to_string(),
            destination: "localhost".to_string(),
            user: Some("testuser".to_string()),
            port: Some("22".to_string()),
            aliases: "".to_string(),
            proxy_command: None,
        }
    }

    #[test]
    fn test_session_creation() {
        let host = create_test_host();
        let config = SessionConfig::default();
        let session = SshSession::new(host.clone(), config);
        
        assert_eq!(session.host.name, "test-host");
        assert_eq!(session.state, ConnectionState::Disconnected);
        assert_eq!(session.activity, ActivityIndicator::None);
        assert!(!session.is_active());
    }

    #[test]
    fn test_display_name_generation() {
        let host = Host {
            name: "very-long-hostname-that-should-be-truncated".to_string(),
            destination: "example.com".to_string(),
            user: None,
            port: None,
            aliases: "".to_string(),
            proxy_command: None,
        };
        
        let display_name = SshSession::generate_display_name(&host);
        assert!(display_name.len() <= 15);
        assert!(display_name.ends_with("..."));
    }

    #[test]
    fn test_terminal_buffer() {
        let mut buffer = TerminalBuffer::new(100);
        
        // Test processing data
        let test_data = b"Hello, World!\n";
        buffer.process_data(test_data);
        
        assert!(buffer.has_new_output());
        
        // Mark as viewed
        buffer.mark_viewed();
        assert!(!buffer.has_new_output());
    }

    #[test]
    fn test_terminal_buffer_scrollback_limit() {
        let mut buffer = TerminalBuffer::new(5); // Small limit for testing
        
        // Add more lines than the limit
        for i in 0..10 {
            let data = format!("Line {}\n", i);
            buffer.process_data(data.as_bytes());
        }
        
        // Should not exceed the limit
        assert!(buffer.scrollback.len() <= 5);
    }

    #[test]
    fn test_search_functionality() {
        let mut buffer = TerminalBuffer::new(100);
        
        // Add some test data
        buffer.process_data(b"First line\n");
        buffer.process_data(b"Second line with pattern\n");
        buffer.process_data(b"Third line\n");
        buffer.process_data(b"Another line with PATTERN\n");
        
        let results = buffer.search("pattern");
        assert_eq!(results.len(), 2); // Should find both lines (case insensitive)
    }
}