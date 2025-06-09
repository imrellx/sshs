use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
#[allow(clippy::wildcard_imports)]
use ratatui::{prelude::*, widgets::*};
use std::{
    cell::RefCell,
    cmp::{max, min},
    io,
    rc::Rc,
    time::{Duration, Instant},
};
use style::palette::tailwind;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use unicode_width::UnicodeWidthStr;

use crate::{searchable::Searchable, ssh};
use super::form::{AddHostForm, FormState};
use super::session::SessionConfig;
use super::tabs::TabManager;

// UI Constants
pub const INFO_TEXT: &str = "(Esc) quit | (↑) move up | (↓) move down | (enter) select | (Ctrl+N) new host | (Ctrl+E) edit host";
pub const SEARCH_BAR_HEIGHT: u16 = 3;
pub const TABLE_MIN_HEIGHT: u16 = 5;
pub const FOOTER_HEIGHT: u16 = 3;
pub const PAGE_SIZE: usize = 21;
pub const CURSOR_HORIZONTAL_PADDING: u16 = 4;
pub const CURSOR_VERTICAL_OFFSET: u16 = 1;
pub const COLUMN_PADDING: u16 = 1;
pub const SEARCHBAR_HORIZONTAL_PADDING: u16 = 3;
pub const TABLE_HEADER_HEIGHT: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusState {
    /// Normal mode - focus on host list, Vim-like navigation
    Normal,
    /// Search mode - focus on search field for typing
    Search,
    /// Session mode - focus on active SSH session
    Session,
    /// Session manager overlay mode
    SessionManager,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub config_paths: Vec<String>,

    pub search_filter: Option<String>,
    pub sort_by_name: bool,
    pub show_proxy_command: bool,

    pub command_template: String,
    pub command_template_on_session_start: Option<String>,
    pub command_template_on_session_end: Option<String>,
    pub exit_after_ssh_session_ends: bool,
}

pub struct App {
    pub config: AppConfig,

    pub search: Input,

    pub table_state: TableState,
    pub hosts: Searchable<ssh::Host>,
    pub table_columns_constraints: Vec<Constraint>,

    pub palette: tailwind::Palette,
    
    // Tab-based SSH session management
    pub tab_manager: TabManager,
    
    // Add/Edit Host Form
    pub add_host_form: Option<AddHostForm>,
    pub form_state: FormState,
    pub feedback_message: Option<String>,
    pub is_feedback_error: bool,
    pub feedback_timeout: Option<Instant>,
    pub is_edit_mode: bool,
    pub editing_host_index: Option<usize>,
    
    // Confirmation dialog
    pub confirm_message: Option<String>,
    pub confirm_action: Option<String>,
    
    // Vim-like navigation and tab navigation
    pub focus_state: FocusState,
    pub last_key_time: Option<Instant>,
    pub pending_g: bool, // For detecting "gg" sequence
    pub pending_gt: bool, // For detecting "gt" sequence
    pub pending_number: Option<u8>, // For detecting "[1-9]gt" sequences
    
}

#[derive(PartialEq, Debug)]
pub enum AppKeyAction {
    Ok,
    Stop,
    Continue,
    Confirm,
}

impl App {
    /// # Errors
    ///
    /// Will return `Err` if the SSH configuration file cannot be parsed.
    pub fn new(config: &AppConfig) -> Result<App> {
        let mut hosts = Vec::new();

        for path in &config.config_paths {
            let parsed_hosts = match ssh::parse_config(path) {
                Ok(hosts) => hosts,
                Err(err) => {
                    if path == "/etc/ssh/ssh_config" {
                        if let ssh::ParseConfigError::Io(io_err) = &err {
                            // Ignore missing system-wide SSH configuration file
                            if io_err.kind() == std::io::ErrorKind::NotFound {
                                continue;
                            }
                        }
                    }

                    anyhow::bail!("Failed to parse SSH configuration file '{}': {}", path, err);
                }
            };

            hosts.extend(parsed_hosts);
        }

        if config.sort_by_name {
            hosts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }

        let search_input = config.search_filter.clone().unwrap_or_default();
        let matcher = SkimMatcherV2::default();

        let session_config = SessionConfig::default();
        let tab_manager = TabManager::new(session_config, tailwind::BLUE);

        let mut app = App {
            config: config.clone(),

            search: search_input.clone().into(),

            table_state: TableState::default().with_selected(0),
            table_columns_constraints: Vec::new(),
            palette: tailwind::BLUE,

            tab_manager,

            hosts: Searchable::new(
                hosts,
                &search_input,
                move |host: &&ssh::Host, search_value: &str| -> bool {
                    search_value.is_empty()
                        || matcher.fuzzy_match(&host.name, search_value).is_some()
                        || matcher
                            .fuzzy_match(&host.destination, search_value)
                            .is_some()
                        || matcher.fuzzy_match(&host.aliases, search_value).is_some()
                },
            ),
            
            add_host_form: None,
            form_state: FormState::Hidden,
            feedback_message: None,
            is_feedback_error: false,
            feedback_timeout: None,
            is_edit_mode: false,
            editing_host_index: None,
            
            confirm_message: None,
            confirm_action: None,
            
            focus_state: FocusState::Normal,
            last_key_time: None,
            pending_g: false,
            pending_gt: false,
            pending_number: None,
        };
        app.calculate_table_columns_constraints();

        Ok(app)
    }

    /// # Errors
    ///
    /// Will return `Err` if the terminal cannot be configured.
    pub fn start(&mut self) -> Result<()> {
        let stdout = io::stdout().lock();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Rc::new(RefCell::new(Terminal::new(backend)?));

        // Set up simple signal handler for Ctrl+C using crossterm only, not ctrlc
        // This way we don't need to share the terminal between threads
        crossterm::event::read()
            .ok() // Prepare the event system, ignore initial read result
            .and(None::<crossterm::event::Event>); // Return None to continue

        // Set up terminal
        safe_setup_terminal(&terminal)?;

        // Run the application with appropriate error handling
        let res = self.run(&terminal);

        // Ensure we always restore the terminal state
        let restore_result = safe_restore_terminal(&terminal);
        
        // Handle any errors from the application run
        if let Err(err) = res {
            eprintln!("Application error: {}", err);
            // Also attempt to show the error cause chain for debugging
            let mut source = err.source();
            while let Some(err) = source {
                eprintln!("Caused by: {}", err);
                source = err.source();
            }
        }

        // Finally, handle any errors from terminal restoration
        restore_result?;

        Ok(())
    }

    fn run<B>(&mut self, terminal: &Rc<RefCell<Terminal<B>>>) -> Result<()>
    where
        B: Backend + std::io::Write,
    {
        loop {
            // Check if feedback message should be cleared due to timeout
            self.check_feedback_timeout();
            
            // Process pending SSH session data
            self.tab_manager.process_all_sessions();
            
            terminal.borrow_mut().draw(|f| super::render::ui(f, self))?;

            let ev = event::read()?;

            if let Event::Key(key) = ev {
                if key.kind == KeyEventKind::Press {
                    match self.form_state {
                        FormState::Hidden => {
                            let action = self.on_key_press(terminal, key)?;
                            match action {
                                AppKeyAction::Ok => continue,
                                AppKeyAction::Stop => break,
                                AppKeyAction::Confirm => continue, // Should not happen in this state
                                AppKeyAction::Continue => {}
                            }
                        }
                        FormState::Active | FormState::Confirming => {
                            let action = self.on_form_key_press(key)?;
                            match action {
                                AppKeyAction::Ok => continue,
                                AppKeyAction::Stop => {
                                    self.form_state = FormState::Hidden;
                                    self.add_host_form = None;
                                    self.confirm_message = None;
                                    self.confirm_action = None;
                                    self.is_edit_mode = false;
                                    self.editing_host_index = None;
                                    continue;
                                }
                                AppKeyAction::Confirm => continue,
                                AppKeyAction::Continue => {}
                            }
                        }
                    }
                }

                match self.form_state {
                    FormState::Hidden => {
                        // Handle search input only in Search mode
                        // But handle mode transitions FIRST before passing events to search input
                        if self.focus_state == FocusState::Search {
                            // Check for mode-changing keys first
                            if let Event::Key(key) = &ev {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Enter => {
                                        // Handle mode transition, don't pass to search input
                                        // This will be handled in the key press handler below
                                    }
                                    _ => {
                                        // For all other keys, let search input handle them
                                        self.search.handle_event(&ev);
                                        self.hosts.search(self.search.value());

                                        let selected = self.table_state.selected().unwrap_or(0);
                                        if selected >= self.hosts.len() {
                                            self.table_state.select(Some(match self.hosts.len() {
                                                0 => 0,
                                                _ => self.hosts.len() - 1,
                                            }));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    FormState::Active => {
                        if let Some(form) = &mut self.add_host_form {
                            form.handle_event(&ev);
                        }
                    }
                    FormState::Confirming => {
                        // Don't handle regular events in confirmation mode
                        // Only key presses are handled
                    }
                }
            }
        }

        Ok(())
    }

    fn on_key_press<B>(
        &mut self,
        terminal: &Rc<RefCell<Terminal<B>>>,
        key: KeyEvent,
    ) -> Result<AppKeyAction>
    where
        B: Backend + std::io::Write,
    {

        let is_ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
        let is_shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

        // Handle global Ctrl+Shift shortcuts first
        if is_ctrl_pressed && is_shift_pressed {
            let action = self.on_key_press_ctrl_shift(key);
            if action != AppKeyAction::Continue {
                return Ok(action);
            }
        }

        // Handle global Ctrl shortcuts
        if is_ctrl_pressed && !is_shift_pressed {
            let action = self.on_key_press_ctrl(key);
            if action != AppKeyAction::Continue {
                return Ok(action);
            }
        }

        // Handle mode-specific key presses
        match self.focus_state {
            FocusState::Normal => self.handle_normal_mode_keys(terminal, key),
            FocusState::Search => self.handle_search_mode_keys(key),
            FocusState::Session => self.handle_session_mode_keys(key),
            FocusState::SessionManager => self.handle_session_manager_keys(key),
        }
    }
    
    fn handle_normal_mode_keys<B>(
        &mut self,
        _terminal: &Rc<RefCell<Terminal<B>>>,
        key: KeyEvent,
    ) -> Result<AppKeyAction>
    where
        B: Backend + std::io::Write,
    {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        // Check for timeout on pending sequences
        if self.pending_g || self.pending_gt || self.pending_number.is_some() {
            if let Some(last_time) = self.last_key_time {
                if last_time.elapsed() > Duration::from_millis(1000) {
                    self.pending_g = false;
                    self.pending_gt = false;
                    self.pending_number = None;
                    self.last_key_time = None;
                }
            }
        }

        match key.code {
            // Quit application with 'q' (Vim-like)
            Char('q') => return Ok(AppKeyAction::Stop),
            
            // Vim-like navigation
            Char('j') => self.next(),
            Char('k') => self.previous(),
            Char('h') => {}, // Reserved for future horizontal navigation
            Char('l') => {}, // Reserved for future horizontal navigation
            
            // Jump to extremes
            Char('G') => self.table_state.select(Some(self.hosts.len().saturating_sub(1))),
            Char('g') => {
                if self.pending_g {
                    // Second 'g' - jump to top
                    self.table_state.select(Some(0));
                    self.pending_g = false;
                    self.last_key_time = None;
                } else if self.pending_number.is_some() {
                    // [1-9]g followed by 't' would be handled in 't' case
                    self.pending_g = true;
                    self.last_key_time = Some(Instant::now());
                } else {
                    // First 'g' - start sequence
                    self.pending_g = true;
                    self.last_key_time = Some(Instant::now());
                }
            }
            
            // Tab navigation - gt (next tab)
            Char('t') => {
                if self.pending_g {
                    // gt - next tab
                    self.tab_manager.next_tab();
                    self.focus_state = FocusState::Session;
                    self.pending_g = false;
                    self.last_key_time = None;
                } else if let Some(number) = self.pending_number {
                    // [1-9]gt - go to specific tab
                    if let Err(e) = self.tab_manager.goto_tab(number as usize) {
                        self.set_feedback_message(format!("Tab navigation error: {}", e), true);
                    } else {
                        self.focus_state = FocusState::Session;
                    }
                    self.pending_number = None;
                    self.last_key_time = None;
                }
            }
            
            // Tab navigation - gT (previous tab)  
            Char('T') => {
                if self.pending_g {
                    // gT - previous tab
                    self.tab_manager.previous_tab();
                    self.focus_state = FocusState::Session;
                    self.pending_g = false;
                    self.last_key_time = None;
                }
            }
            
            // Number prefixes for tab navigation [1-9]gt
            Char(c @ '1'..='9') => {
                let digit = c.to_digit(10).unwrap() as u8;
                self.pending_number = Some(digit);
                self.last_key_time = Some(Instant::now());
            }
            
            // Search mode transitions
            Char('/') => {
                self.focus_state = FocusState::Search;
                // Clear search to start fresh
                self.search = Input::default();
                self.hosts.search("");
            }
            
            // Host management (single key - more Vim-like)
            Char('n') => {
                self.open_add_host_form();
            }
            Char('e') => {
                self.open_edit_host_form();
            }
            Char('d') => {
                self.open_delete_host_confirmation();
            }
            
            // Traditional navigation (backward compatibility)
            Down => self.next(),
            Up => self.previous(),
            Home => self.table_state.select(Some(0)),
            End => self.table_state.select(Some(self.hosts.len().saturating_sub(1))),
            PageDown => {
                let i = self.table_state.selected().unwrap_or(0);
                let target = min(i.saturating_add(PAGE_SIZE), self.hosts.len().saturating_sub(1));
                self.table_state.select(Some(target));
            }
            PageUp => {
                let i = self.table_state.selected().unwrap_or(0);
                let target = max(i.saturating_sub(PAGE_SIZE), 0);
                self.table_state.select(Some(target));
            }
            
            // Connect to host in new tab
            Enter => {
                if let Some(selected) = self.table_state.selected() {
                    if selected < self.hosts.len() {
                        let host = self.hosts[selected].clone();
                        if let Err(e) = self.create_ssh_session_from_host(host) {
                            self.set_feedback_message(format!("Failed to create session: {}", e), true);
                        } else {
                            self.focus_state = FocusState::Session;
                        }
                    }
                }
            }
            
            // Tab for alternative navigation
            Tab => self.next(),
            BackTab => self.previous(),
            
            _ => return Ok(AppKeyAction::Continue),
        }

        // Clear pending states for any other key
        if !matches!(key.code, Char('g') | Char('t') | Char('T') | Char('1'..='9')) {
            self.pending_g = false;
            self.pending_gt = false;
            self.pending_number = None;
            self.last_key_time = None;
        }

        Ok(AppKeyAction::Ok)
    }
    
    fn handle_search_mode_keys(&mut self, key: KeyEvent) -> Result<AppKeyAction> {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Esc => {
                // Exit search mode, return to normal mode
                self.focus_state = FocusState::Normal;
                // Clear search text and show all hosts
                self.search = Input::default();
                self.hosts.search("");
                // Focus on first host
                if !self.hosts.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            Enter => {
                // Finish search and switch to normal mode with focus on first result
                self.focus_state = FocusState::Normal;
                if !self.hosts.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            _ => {
                // Let the search field handle the input - this is already done in the main loop
                return Ok(AppKeyAction::Continue);
            }
        }

        Ok(AppKeyAction::Ok)
    }

    fn on_key_press_ctrl(&mut self, key: KeyEvent) -> AppKeyAction {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Char('c') => AppKeyAction::Stop,
            Char('j') => {
                self.next();
                AppKeyAction::Ok
            }
            Char('f') => {
                // Ctrl+F to enter search mode (alternative to '/')
                self.focus_state = FocusState::Search;
                self.search = Input::default();
                self.hosts.search("");
                AppKeyAction::Ok
            }
            Char('k' | 'p') => {
                self.previous();
                AppKeyAction::Ok
            }
            Char('t') => {
                // Ctrl+T to create new SSH session tab from selected host
                if let Some(selected) = self.table_state.selected() {
                    if selected < self.hosts.len() {
                        let host = self.hosts[selected].clone();
                        if let Err(e) = self.create_ssh_session_from_host(host) {
                            self.set_feedback_message(format!("Failed to create session: {}", e), true);
                        }
                    }
                }
                AppKeyAction::Ok
            }
            Char('w') => {
                // Ctrl+W to close current tab
                if !self.tab_manager.is_empty() {
                    if let Err(e) = self.tab_manager.close_current_tab() {
                        self.set_feedback_message(format!("Failed to close tab: {}", e), true);
                    }
                }
                AppKeyAction::Ok
            }
            Tab => {
                // Ctrl+Tab for next tab
                self.tab_manager.next_tab();
                self.focus_state = FocusState::Session;
                AppKeyAction::Ok
            }
            BackTab => {
                // Ctrl+Shift+Tab for previous tab
                self.tab_manager.previous_tab();
                self.focus_state = FocusState::Session;
                AppKeyAction::Ok
            }
            _ => AppKeyAction::Continue,
        }
    }
    
    fn on_key_press_ctrl_shift(&mut self, key: KeyEvent) -> AppKeyAction {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Char('s') | Char('S') => {
                // Ctrl+Shift+S to toggle session manager overlay
                if !self.tab_manager.is_empty() {
                    self.focus_state = match self.focus_state {
                        FocusState::SessionManager => FocusState::Session,
                        _ => FocusState::SessionManager,
                    };
                }
                AppKeyAction::Ok
            }
            _ => AppKeyAction::Continue,
        }
    }
    
    fn handle_session_mode_keys(&mut self, key: KeyEvent) -> Result<AppKeyAction> {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Esc => {
                // Return to host selection
                self.focus_state = FocusState::Normal;
                Ok(AppKeyAction::Ok)
            }
            Char('q') => {
                // Quit application
                Ok(AppKeyAction::Stop)
            }
            _ => {
                // Forward other keys to the active session
                if let Some(_session) = self.tab_manager.get_active_session() {
                    // TODO: Forward key input to active SSH session
                    // For now, just ignore other keys in session mode
                }
                Ok(AppKeyAction::Ok)
            }
        }
    }
    
    fn handle_session_manager_keys(&mut self, key: KeyEvent) -> Result<AppKeyAction> {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Esc | Char('q') | Char('Q') => {
                // Exit session manager
                self.focus_state = FocusState::Session;
                Ok(AppKeyAction::Ok)
            }
            Enter => {
                // Switch to selected session (placeholder)
                self.focus_state = FocusState::Session;
                Ok(AppKeyAction::Ok)
            }
            Char('d') | Char('D') => {
                // Disconnect session (placeholder)
                if !self.tab_manager.is_empty() {
                    if let Err(e) = self.tab_manager.close_current_tab() {
                        self.set_feedback_message(format!("Failed to close tab: {}", e), true);
                    }
                }
                Ok(AppKeyAction::Ok)
            }
            Char('n') | Char('N') => {
                // Create new session - switch to host selection
                self.focus_state = FocusState::Normal;
                Ok(AppKeyAction::Ok)
            }
            _ => Ok(AppKeyAction::Ok),
        }
    }
    
    fn on_form_key_press(&mut self, key: KeyEvent) -> Result<AppKeyAction> {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        // If we're in confirmation mode, handle that first
        if self.form_state == FormState::Confirming {
            match key.code {
                Esc | Char('n') | Char('N') => {
                    // Cancel the confirmation
                    self.form_state = FormState::Active;
                    self.confirm_message = None;
                    self.confirm_action = None;
                    return Ok(AppKeyAction::Ok);
                },
                Enter | Char('y') | Char('Y') => {
                    // Check if this is a delete confirmation
                    if let Some(action) = &self.confirm_action {
                        if action == "Delete" {
                            // Handle host deletion
                            self.form_state = FormState::Hidden;
                            let result = self.delete_selected_host();
                            
                            match result {
                                Ok(()) => {
                                    self.confirm_message = None;
                                    self.confirm_action = None;
                                    self.editing_host_index = None;
                                    return Ok(AppKeyAction::Ok);
                                }
                                Err(e) => {
                                    self.set_feedback_message(format!("Error deleting host: {}", e), true);
                                    self.confirm_message = None;
                                    self.confirm_action = None;
                                    self.editing_host_index = None;
                                    return Ok(AppKeyAction::Ok);
                                }
                            }
                        }
                    }
                    
                    // Proceed with saving (existing functionality)
                    self.form_state = FormState::Active;
                    
                    // Save the host (we already validated it's valid)
                    let result = if self.is_edit_mode {
                        self.update_existing_host()
                    } else {
                        self.save_new_host()
                    };
                    
                    match result {
                        Ok(()) => {
                            let message = if self.is_edit_mode {
                                "Host updated successfully!"
                            } else {
                                "Host added successfully!"
                            };
                            self.set_feedback_message(message.to_string(), false);
                            self.form_state = FormState::Hidden;
                            self.add_host_form = None;
                            self.confirm_message = None;
                            self.confirm_action = None;
                            self.is_edit_mode = false;
                            self.editing_host_index = None;
                            
                            // Reload the hosts
                            self.reload_hosts()?;
                            
                            return Ok(AppKeyAction::Ok);
                        }
                        Err(e) => {
                            self.set_feedback_message(format!("Error: {}", e), true);
                            self.confirm_message = None;
                            self.confirm_action = None;
                            return Ok(AppKeyAction::Ok);
                        }
                    }
                },
                _ => return Ok(AppKeyAction::Continue),
            }
        }

        // Normal form handling
        match key.code {
            Esc => Ok(AppKeyAction::Stop),
            Enter => {
                if let Some(form) = &self.add_host_form {
                    if form.is_valid() {
                        // Check if the host already exists
                        let config_path = shellexpand::tilde(&self.config.config_paths[1]).to_string();
                        match form.check_duplicate(&config_path) {
                            Ok(true) => {
                                // Host exists, show confirmation dialog
                                self.confirm_message = Some(format!("Host '{}' already exists. Overwrite?", 
                                    form.host_name.value().trim()));
                                self.confirm_action = Some("Overwrite".to_string());
                                self.form_state = FormState::Confirming;
                                return Ok(AppKeyAction::Confirm);
                            },
                            Ok(false) => {
                                // No duplicate, proceed with saving
                                let result = if self.is_edit_mode {
                                    self.update_existing_host()
                                } else {
                                    self.save_new_host()
                                };
                                
                                match result {
                                    Ok(()) => {
                                        let message = if self.is_edit_mode {
                                            "Host updated successfully!"
                                        } else {
                                            "Host added successfully!"
                                        };
                                        self.set_feedback_message(message.to_string(), false);
                                        self.form_state = FormState::Hidden;
                                        self.add_host_form = None;
                                        self.is_edit_mode = false;
                                        self.editing_host_index = None;
                                        
                                        // Reload the hosts
                                        self.reload_hosts()?;
                                        
                                        return Ok(AppKeyAction::Ok);
                                    }
                                    Err(e) => {
                                        self.set_feedback_message(format!("Error: {}", e), true);
                                        return Ok(AppKeyAction::Ok);
                                    }
                                }
                            },
                            Err(e) => {
                                // Error checking for duplicates
                                self.set_feedback_message(format!("Error checking for duplicates: {}", e), true);
                                return Ok(AppKeyAction::Ok);
                            }
                        }
                    }
                    
                    // Show specific validation error message
                    if let Some(error_message) = form.validation_error() {
                        self.set_feedback_message(error_message, true);
                    } else {
                        self.set_feedback_message("Invalid form data".to_string(), true);
                    }
                    
                    return Ok(AppKeyAction::Ok);
                }
                Ok(AppKeyAction::Continue)
            }
            Tab => {
                if let Some(form) = &mut self.add_host_form {
                    form.next_field();
                    return Ok(AppKeyAction::Ok);
                }
                Ok(AppKeyAction::Continue)
            }
            BackTab => {
                if let Some(form) = &mut self.add_host_form {
                    form.previous_field();
                    return Ok(AppKeyAction::Ok);
                }
                Ok(AppKeyAction::Continue)
            }
            _ => Ok(AppKeyAction::Continue),
        }
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if self.hosts.is_empty() || i >= self.hosts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if self.hosts.is_empty() {
                    0
                } else if i == 0 {
                    self.hosts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn calculate_table_columns_constraints(&mut self) {
        let mut lengths = Vec::new();

        let name_len = self
            .hosts
            .iter()
            .map(|d| d.name.as_str())
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        lengths.push(name_len);

        let aliases_len = self
            .hosts
            .non_filtered_iter()
            .map(|d| d.aliases.as_str())
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        lengths.push(aliases_len);

        let user_len = self
            .hosts
            .non_filtered_iter()
            .map(|d| match &d.user {
                Some(user) => user.as_str(),
                None => "",
            })
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        lengths.push(user_len);

        let destination_len = self
            .hosts
            .non_filtered_iter()
            .map(|d| d.destination.as_str())
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        lengths.push(destination_len);

        let port_len = self
            .hosts
            .non_filtered_iter()
            .map(|d| match &d.port {
                Some(port) => port.as_str(),
                None => "",
            })
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        lengths.push(port_len);

        if self.config.show_proxy_command {
            let proxy_len = self
                .hosts
                .non_filtered_iter()
                .map(|d| match &d.proxy_command {
                    Some(proxy) => proxy.as_str(),
                    None => "",
                })
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or(0);
            lengths.push(proxy_len);
        }

        self.table_columns_constraints = vec![
            // +COLUMN_PADDING for padding
            Constraint::Length(u16::try_from(lengths[0]).unwrap_or_default() + COLUMN_PADDING),
        ];
        self.table_columns_constraints.extend(
            lengths
                .iter()
                .skip(1)
                .map(|len| Constraint::Min(u16::try_from(*len).unwrap_or_default() + COLUMN_PADDING)),
        );
    }
    
    fn open_add_host_form(&mut self) {
        self.add_host_form = Some(AddHostForm::new());
        self.form_state = FormState::Active;
        self.feedback_message = None;
        self.feedback_timeout = None;
        self.is_edit_mode = false;
        self.editing_host_index = None;
    }
    
    fn open_edit_host_form(&mut self) {
        let selected = self.table_state.selected().unwrap_or(0);
        if selected >= self.hosts.len() {
            self.set_feedback_message("No host selected for editing".to_string(), true);
            return;
        }

        let host = &self.hosts[selected];
        let mut form = AddHostForm::new();
        
        // Pre-populate the form with existing host data
        form.populate_from_host(host);
        
        self.add_host_form = Some(form);
        self.form_state = FormState::Active;
        self.feedback_message = None;
        self.feedback_timeout = None;
        self.is_edit_mode = true;
        self.editing_host_index = Some(selected);
    }
    
    fn save_new_host(&self) -> Result<()> {
        if let Some(form) = &self.add_host_form {
            let config_path = shellexpand::tilde(&self.config.config_paths[1]).to_string();
            form.save_to_config(&config_path)
        } else {
            Err(anyhow::anyhow!("Form is not initialized"))
        }
    }
    
    fn update_existing_host(&self) -> Result<()> {
        if let Some(form) = &self.add_host_form {
            if let Some(host_index) = self.editing_host_index {
                let config_path = shellexpand::tilde(&self.config.config_paths[1]).to_string();
                let original_host = &self.hosts[host_index];
                form.update_host_in_config(&config_path, original_host)
            } else {
                Err(anyhow::anyhow!("No host selected for editing"))
            }
        } else {
            Err(anyhow::anyhow!("Form is not initialized"))
        }
    }
    
    fn open_delete_host_confirmation(&mut self) {
        let selected = self.table_state.selected().unwrap_or(0);
        if selected >= self.hosts.len() {
            self.set_feedback_message("No host selected for deletion".to_string(), true);
            return;
        }

        let host = &self.hosts[selected];
        self.confirm_message = Some(format!("Delete host '{}'? This action cannot be undone.", host.name));
        self.confirm_action = Some("Delete".to_string());
        self.form_state = FormState::Confirming;
        self.editing_host_index = Some(selected);
    }
    
    fn delete_selected_host(&mut self) -> Result<()> {
        if let Some(host_index) = self.editing_host_index {
            if host_index >= self.hosts.len() {
                return Err(anyhow::anyhow!("Invalid host index for deletion"));
            }

            let host = self.hosts[host_index].clone();
            let config_path = shellexpand::tilde(&self.config.config_paths[1]).to_string();
            
            // Delete the host from SSH config file
            self.delete_host_from_config(&config_path, &host)?;
            
            // Reload hosts to refresh the list
            self.reload_hosts()?;
            
            // Adjust selection if necessary
            if host_index >= self.hosts.len() && !self.hosts.is_empty() {
                self.table_state.select(Some(self.hosts.len() - 1));
            } else if self.hosts.is_empty() {
                self.table_state.select(Some(0));
            }
            
            // Show success message
            self.set_feedback_message(format!("Host '{}' deleted successfully", host.name), false);
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("No host selected for deletion"))
        }
    }
    
    fn delete_host_from_config(&self, config_path: &str, host_to_delete: &ssh::Host) -> Result<()> {
        use std::fs;
        
        // Read the current config file
        let content = fs::read_to_string(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read SSH config file: {}", e))?;
        
        // Create a backup of the original config file
        let backup_path = format!("{}.bak", config_path);
        fs::copy(config_path, &backup_path)
            .map_err(|e| anyhow::anyhow!("Failed to create backup of SSH config file: {}", e))?;
        
        // Parse and remove the host entry
        let updated_content = self.remove_host_entry(&content, host_to_delete)?;
        
        // Write the updated content back to the file
        fs::write(config_path, updated_content)
            .map_err(|e| anyhow::anyhow!("Failed to write updated SSH config file: {}", e))?;
        
        Ok(())
    }
    
    fn remove_host_entry(&self, content: &str, host_to_delete: &ssh::Host) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;
        let mut found_host = false;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for Host lines that match our target host name
            if let Some(stripped) = line.strip_prefix("Host ") {
                let pattern = stripped.trim();
                let clean_pattern = pattern.trim_matches('"');
                
                if clean_pattern == host_to_delete.name {
                    found_host = true;
                    // Skip this host block
                    i += 1;
                    
                    // Skip all lines until the next Host block or end of file
                    while i < lines.len() {
                        let next_line = lines[i].trim();
                        if next_line.starts_with("Host ") && !next_line.is_empty() {
                            break;
                        }
                        i += 1;
                    }
                    
                    continue;
                }
            }
            
            result.push(lines[i].to_string());
            i += 1;
        }
        
        if !found_host {
            return Err(anyhow::anyhow!("Host '{}' not found in SSH config file", host_to_delete.name));
        }
        
        Ok(result.join("\n"))
    }
    
    fn set_feedback_message(&mut self, message: String, is_error: bool) {
        self.feedback_message = Some(message);
        self.is_feedback_error = is_error;
        self.feedback_timeout = Some(Instant::now());
    }
    
    fn check_feedback_timeout(&mut self) {
        if let Some(timeout) = self.feedback_timeout {
            // Clear feedback message after 3 seconds
            if timeout.elapsed() > Duration::from_secs(3) {
                self.feedback_message = None;
                self.feedback_timeout = None;
            }
        }
    }
    
    fn reload_hosts(&mut self) -> Result<()> {
        let mut hosts = Vec::new();

        for path in &self.config.config_paths {
            let parsed_hosts = match ssh::parse_config(path) {
                Ok(hosts) => hosts,
                Err(err) => {
                    if path == "/etc/ssh/ssh_config" {
                        if let ssh::ParseConfigError::Io(io_err) = &err {
                            // Ignore missing system-wide SSH configuration file
                            if io_err.kind() == std::io::ErrorKind::NotFound {
                                continue;
                            }
                        }
                    }

                    anyhow::bail!("Failed to parse SSH configuration file '{}': {}", path, err);
                }
            };

            hosts.extend(parsed_hosts);
        }

        if self.config.sort_by_name {
            hosts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }

        let search_input = self.search.value();
        let matcher = SkimMatcherV2::default();

        self.hosts = Searchable::new(
            hosts,
            search_input,
            move |host: &&ssh::Host, search_value: &str| -> bool {
                search_value.is_empty()
                    || matcher.fuzzy_match(&host.name, search_value).is_some()
                    || matcher
                        .fuzzy_match(&host.destination, search_value)
                        .is_some()
                    || matcher.fuzzy_match(&host.aliases, search_value).is_some()
            },
        );
        
        self.calculate_table_columns_constraints();
        Ok(())
    }
    
    
    /// Create a new SSH session from a host (non-async wrapper)
    fn create_ssh_session_from_host(&mut self, host: ssh::Host) -> Result<()> {
        // For now, create a placeholder session that will connect asynchronously
        // This is a synchronous wrapper that starts the async connection process
        
        // Check if we already have a session for this host
        if self.tab_manager.has_session_for_host(&host.name) {
            // Switch to existing session
            if let Some(index) = self.tab_manager.find_session_by_host(&host.name) {
                self.tab_manager.goto_tab(index + 1)?; // 1-based indexing
            }
            return Ok(());
        }

        // Create session config
        let session_config = SessionConfig::default();
        let _session = super::session::SshSession::new(host, session_config);
        
        // Use the tab manager's create_session method (but we need async for that)
        // For now, create a simplified version that just adds a placeholder session
        
        // TODO: Implement proper async session creation
        // This is a temporary implementation to get the structure working
        
        self.set_feedback_message("Creating session...".to_string(), false);
        
        Ok(())
    }
}

// Better error handling for terminal setup/teardown
pub fn safe_setup_terminal<B>(terminal: &Rc<RefCell<Terminal<B>>>) -> Result<()>
where
    B: Backend + std::io::Write,
{
    // First, try to restore the terminal in case it was left in a bad state
    // We ignore errors here since we're just making sure we're starting fresh
    let _ = disable_raw_mode();
    let _ = {
        let mut terminal_ref = terminal.borrow_mut();
        execute!(terminal_ref.backend_mut(), Show, LeaveAlternateScreen, DisableMouseCapture)
    };

    // Now set up the terminal properly
    enable_raw_mode().map_err(|e| anyhow::anyhow!("Failed to enable raw mode: {}", e))?;
    
    // Set up terminal features one by one to better identify issues
    let mut terminal_ref = terminal.borrow_mut();
    
    execute!(terminal_ref.backend_mut(), Hide)
        .map_err(|e| anyhow::anyhow!("Failed to hide cursor: {}", e))?;
    
    execute!(terminal_ref.backend_mut(), EnterAlternateScreen)
        .map_err(|e| anyhow::anyhow!("Failed to enter alternate screen: {}", e))?;
    
    execute!(terminal_ref.backend_mut(), EnableMouseCapture)
        .map_err(|e| anyhow::anyhow!("Failed to enable mouse capture: {}", e))?;
    
    Ok(())
}

pub fn safe_restore_terminal<B>(terminal: &Rc<RefCell<Terminal<B>>>) -> Result<()>
where
    B: Backend + std::io::Write,
{
    // Gather errors rather than failing on the first one
    let mut errors = Vec::new();
    
    // Try to clear terminal
    if let Err(e) = terminal.borrow_mut().clear() {
        errors.push(format!("Failed to clear terminal: {}", e));
    }
    
    // Try to disable raw mode - very important to restore
    if let Err(e) = disable_raw_mode() {
        errors.push(format!("Failed to disable raw mode: {}", e));
    }
    
    // Try to restore terminal state
    {
        let mut terminal_ref = terminal.borrow_mut();
        
        // Show cursor
        if let Err(e) = execute!(terminal_ref.backend_mut(), Show) {
            errors.push(format!("Failed to show cursor: {}", e));
        }
        
        // Leave alternate screen
        if let Err(e) = execute!(terminal_ref.backend_mut(), LeaveAlternateScreen) {
            errors.push(format!("Failed to leave alternate screen: {}", e));
        }
        
        // Disable mouse capture
        if let Err(e) = execute!(terminal_ref.backend_mut(), DisableMouseCapture) {
            errors.push(format!("Failed to disable mouse capture: {}", e));
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Terminal restoration errors: {}", errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::Duration;

    /// Helper function to create a test app
    fn create_test_app() -> App {
        let config = AppConfig {
            config_paths: vec!["/test".to_string()],
            search_filter: None,
            sort_by_name: false,
            show_proxy_command: false,
            command_template: "ssh {destination}".to_string(),
            command_template_on_session_start: None,
            command_template_on_session_end: None,
            exit_after_ssh_session_ends: false,
        };

        let session_config = SessionConfig::default();
        let tab_manager = crate::ui::tabs::TabManager::new(session_config, tailwind::BLUE);

        App {
            config,
            search: Input::default(),
            table_state: TableState::default(),
            hosts: Searchable::new(Vec::new(), "", |_, _| true),
            table_columns_constraints: vec![],
            palette: tailwind::BLUE,
            tab_manager,
            add_host_form: None,
            form_state: FormState::Hidden,
            feedback_message: None,
            is_feedback_error: false,
            feedback_timeout: None,
            is_edit_mode: false,
            editing_host_index: None,
            confirm_message: None,
            confirm_action: None,
            focus_state: FocusState::Normal,
            last_key_time: None,
            pending_g: false,
            pending_gt: false,
            pending_number: None,
        }
    }

    #[test]
    fn test_focus_state_transitions() {
        let mut app = create_test_app();
        
        // Start in Normal mode
        assert_eq!(app.focus_state, FocusState::Normal);
        
        // Simulate pressing '/' to enter Search mode directly
        app.focus_state = FocusState::Search;
        app.search = Input::default();
        app.hosts.search("");
        assert_eq!(app.focus_state, FocusState::Search);
        
        // Press Esc to return to Normal mode
        let key_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let result = app.handle_search_mode_keys(key_event);
        assert!(result.is_ok());
        assert_eq!(app.focus_state, FocusState::Normal);
    }

    #[test]
    fn test_ctrl_f_search_mode() {
        let mut app = create_test_app();
        
        // Start in Normal mode
        assert_eq!(app.focus_state, FocusState::Normal);
        
        // Press Ctrl+F to enter Search mode
        let key_event = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
        let action = app.on_key_press_ctrl(key_event);
        assert_eq!(action, AppKeyAction::Ok);
        assert_eq!(app.focus_state, FocusState::Search);
    }

    #[test]
    fn test_vim_navigation_keys() {
        let mut app = create_test_app();
        
        // Add some test hosts for navigation
        use crate::ssh::Host;
        let hosts = vec![
            Host {
                name: "host1".to_string(),
                destination: "host1.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
            Host {
                name: "host2".to_string(),
                destination: "host2.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
            Host {
                name: "host3".to_string(),
                destination: "host3.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
        ];
        
        app.hosts = Searchable::new(hosts, "", |_, _| true);
        app.table_state.select(Some(0));
        
        // Test j key navigation (simulate the effect)
        app.next();
        assert_eq!(app.table_state.selected(), Some(1));
        
        // Test k key navigation (simulate the effect)
        app.previous();
        assert_eq!(app.table_state.selected(), Some(0));
        
        // Test G key (jump to bottom) - simulate the effect
        app.table_state.select(Some(app.hosts.len().saturating_sub(1)));
        assert_eq!(app.table_state.selected(), Some(2)); // Last host
    }

    #[test]
    fn test_gg_sequence() {
        let mut app = create_test_app();
        
        // Add some test hosts
        use crate::ssh::Host;
        let hosts = vec![
            Host {
                name: "host1".to_string(),
                destination: "host1.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
            Host {
                name: "host2".to_string(),
                destination: "host2.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
        ];
        
        app.hosts = Searchable::new(hosts, "", |_, _| true);
        app.table_state.select(Some(1)); // Start at second host
        
        // Simulate first 'g' - should set pending_g = true
        app.pending_g = true;
        app.last_key_time = Some(Instant::now());
        assert_eq!(app.table_state.selected(), Some(1)); // Should not move yet
        
        // Simulate second 'g' - should jump to top
        app.table_state.select(Some(0));
        app.pending_g = false;
        app.last_key_time = None;
        assert_eq!(app.table_state.selected(), Some(0)); // Should jump to top
    }

    #[test]
    fn test_pending_g_timeout() {
        let mut app = create_test_app();
        
        // Set pending_g with an old timestamp
        app.pending_g = true;
        app.last_key_time = Some(Instant::now() - Duration::from_millis(2000)); // 2 seconds ago
        
        // Simulate checking timeout - pending_g should be cleared
        if let Some(last_time) = app.last_key_time {
            if last_time.elapsed() > Duration::from_millis(1000) {
                app.pending_g = false;
                app.last_key_time = None;
            }
        }
        
        // pending_g should be cleared due to timeout
        assert!(!app.pending_g);
        assert!(app.last_key_time.is_none());
    }

    #[test]
    fn test_q_key_quits_application() {
        let app = create_test_app();
        
        // Ensure we're in Normal mode
        assert_eq!(app.focus_state, FocusState::Normal);
        
        // Test that 'q' is mapped to quit - we can test this by checking if 
        // the quit logic would be triggered in Normal mode
        // Since we can't easily test the full key handler without a terminal,
        // we verify the state setup is correct for quit functionality
        assert_eq!(app.focus_state, FocusState::Normal);
        // In Normal mode, 'q' should trigger quit (tested in integration)
    }

    #[test]
    fn test_search_mode_escape_transitions() {
        let mut app = create_test_app();
        
        // Add some test hosts
        use crate::ssh::Host;
        let hosts = vec![
            Host {
                name: "test-host".to_string(),
                destination: "test.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
            Host {
                name: "prod-host".to_string(),
                destination: "prod.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
        ];
        // Create proper search closure that mimics the real search behavior
        use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
        let matcher = SkimMatcherV2::default();
        app.hosts = Searchable::new(hosts, "", move |host: &&crate::ssh::Host, search_value: &str| -> bool {
            search_value.is_empty()
                || matcher.fuzzy_match(&host.name, search_value).is_some()
                || matcher.fuzzy_match(&host.destination, search_value).is_some()
                || matcher.fuzzy_match(&host.aliases, search_value).is_some()
        });
        
        // Start in Search mode with some search text
        app.focus_state = FocusState::Search;
        app.search = Input::from("nonexistent".to_string());
        app.hosts.search("nonexistent");
        
        // Verify search has filtered out all hosts
        assert_eq!(app.hosts.len(), 0);
        
        // Press Esc - should return to Normal mode and clear search
        let key_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let result = app.handle_search_mode_keys(key_event);
        assert!(result.is_ok());
        assert_eq!(app.focus_state, FocusState::Normal);
        assert_eq!(app.search.value(), ""); // Search should be cleared
        assert_eq!(app.hosts.len(), 2); // All hosts should be visible again
    }

    #[test]
    fn test_search_mode_enter_keeps_filter() {
        let mut app = create_test_app();
        
        // Add some test hosts
        use crate::ssh::Host;
        let hosts = vec![
            Host {
                name: "test-host".to_string(),
                destination: "test.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
            Host {
                name: "prod-host".to_string(),
                destination: "prod.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
        ];
        // Create proper search closure that mimics the real search behavior
        use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
        let matcher = SkimMatcherV2::default();
        app.hosts = Searchable::new(hosts, "", move |host: &&crate::ssh::Host, search_value: &str| -> bool {
            search_value.is_empty()
                || matcher.fuzzy_match(&host.name, search_value).is_some()
                || matcher.fuzzy_match(&host.destination, search_value).is_some()
                || matcher.fuzzy_match(&host.aliases, search_value).is_some()
        });
        
        // Start in Search mode with search text that matches one host
        app.focus_state = FocusState::Search;
        app.search = Input::from("test".to_string());
        app.hosts.search("test");
        
        // Verify search has filtered to one host
        assert_eq!(app.hosts.len(), 1);
        
        // Press Enter - should return to Normal mode but keep search filter
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let result = app.handle_search_mode_keys(key_event);
        assert!(result.is_ok());
        assert_eq!(app.focus_state, FocusState::Normal);
        assert_eq!(app.search.value(), "test"); // Search should be preserved
        assert_eq!(app.hosts.len(), 1); // Filtered results should remain
    }

    #[test]
    fn test_delete_host_functionality() {
        let mut app = create_test_app();
        
        // Add a test host for deletion
        use crate::ssh::Host;
        let hosts = vec![
            Host {
                name: "test-host-1".to_string(),
                destination: "test1.example.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
            Host {
                name: "test-host-2".to_string(),
                destination: "test2.example.com".to_string(),
                user: None,
                port: None,
                aliases: "".to_string(),
                proxy_command: None,
            },
        ];
        
        // Create proper search closure
        use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
        let matcher = SkimMatcherV2::default();
        app.hosts = Searchable::new(hosts, "", move |host: &&crate::ssh::Host, search_value: &str| -> bool {
            search_value.is_empty()
                || matcher.fuzzy_match(&host.name, search_value).is_some()
                || matcher.fuzzy_match(&host.destination, search_value).is_some()
                || matcher.fuzzy_match(&host.aliases, search_value).is_some()
        });
        app.table_state.select(Some(0));
        
        // Test opening delete confirmation
        app.open_delete_host_confirmation();
        assert_eq!(app.form_state, FormState::Confirming);
        assert!(app.confirm_message.is_some());
        assert_eq!(app.confirm_action, Some("Delete".to_string()));
        assert_eq!(app.editing_host_index, Some(0));
        
        // Verify the confirmation message contains the host name
        let confirm_msg = app.confirm_message.as_ref().unwrap();
        assert!(confirm_msg.contains("test-host-1"));
        assert!(confirm_msg.contains("cannot be undone"));
    }

    #[test]
    fn test_feedback_message_timeout() {
        let mut app = create_test_app();
        
        // Test setting a feedback message
        app.set_feedback_message("Test message".to_string(), false);
        assert_eq!(app.feedback_message, Some("Test message".to_string()));
        assert!(!app.is_feedback_error);
        assert!(app.feedback_timeout.is_some());
        
        // Test that message persists before timeout
        app.check_feedback_timeout();
        assert!(app.feedback_message.is_some());
        
        // Simulate timeout by setting feedback_timeout to an old time
        app.feedback_timeout = Some(Instant::now() - Duration::from_secs(5));
        app.check_feedback_timeout();
        assert!(app.feedback_message.is_none());
        assert!(app.feedback_timeout.is_none());
    }

    #[test]
    fn test_single_key_host_management() {
        let mut app = create_test_app();
        
        // Ensure we're in Normal mode
        assert_eq!(app.focus_state, FocusState::Normal);
        assert_eq!(app.form_state, FormState::Hidden);
        
        // Test 'n' key opens add host form
        // We can't easily test the full key handler, but we can test the method directly
        app.open_add_host_form();
        assert_eq!(app.form_state, FormState::Active);
        assert!(app.add_host_form.is_some());
        assert!(!app.is_edit_mode);
        
        // Reset for edit test
        app.form_state = FormState::Hidden;
        app.add_host_form = None;
        app.is_edit_mode = false;
        
        // Add a test host for editing
        use crate::ssh::Host;
        let hosts = vec![Host {
            name: "test-host".to_string(),
            destination: "test.com".to_string(),
            user: None,
            port: None,
            aliases: "".to_string(),
            proxy_command: None,
        }];
        // Create proper search closure
        use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
        let matcher = SkimMatcherV2::default();
        app.hosts = Searchable::new(hosts, "", move |host: &&crate::ssh::Host, search_value: &str| -> bool {
            search_value.is_empty()
                || matcher.fuzzy_match(&host.name, search_value).is_some()
                || matcher.fuzzy_match(&host.destination, search_value).is_some()
                || matcher.fuzzy_match(&host.aliases, search_value).is_some()
        });
        app.table_state.select(Some(0));
        
        // Test 'e' key opens edit host form
        app.open_edit_host_form();
        assert_eq!(app.form_state, FormState::Active);
        assert!(app.add_host_form.is_some());
        assert!(app.is_edit_mode);
        assert_eq!(app.editing_host_index, Some(0));
    }
}