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
};
use style::palette::tailwind;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use unicode_width::UnicodeWidthStr;

use crate::{searchable::Searchable, ssh};
use super::form::{AddHostForm, FormState};

// UI Constants
pub const INFO_TEXT: &str = "(Esc) quit | (↑) move up | (↓) move down | (enter) select | (Ctrl+N) new host";
pub const SEARCH_BAR_HEIGHT: u16 = 3;
pub const TABLE_MIN_HEIGHT: u16 = 5;
pub const FOOTER_HEIGHT: u16 = 3;
pub const PAGE_SIZE: usize = 21;
pub const CURSOR_HORIZONTAL_PADDING: u16 = 4;
pub const CURSOR_VERTICAL_OFFSET: u16 = 1;
pub const COLUMN_PADDING: u16 = 1;
pub const SEARCHBAR_HORIZONTAL_PADDING: u16 = 3;
pub const TABLE_HEADER_HEIGHT: u16 = 1;

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
    
    // Add Host Form
    pub add_host_form: Option<AddHostForm>,
    pub form_state: FormState,
    pub feedback_message: Option<String>,
    pub is_feedback_error: bool,
}

#[derive(PartialEq)]
pub enum AppKeyAction {
    Ok,
    Stop,
    Continue,
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

        let mut app = App {
            config: config.clone(),

            search: search_input.clone().into(),

            table_state: TableState::default().with_selected(0),
            table_columns_constraints: Vec::new(),
            palette: tailwind::BLUE,

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
            .and_then(|_| None::<crossterm::event::Event>); // Return None to continue

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
                                AppKeyAction::Continue => {}
                            }
                        }
                        FormState::Active => {
                            let action = self.on_form_key_press(key)?;
                            match action {
                                AppKeyAction::Ok => continue,
                                AppKeyAction::Stop => {
                                    self.form_state = FormState::Hidden;
                                    self.add_host_form = None;
                                    continue;
                                }
                                AppKeyAction::Continue => {}
                            }
                        }
                    }
                }

                match self.form_state {
                    FormState::Hidden => {
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
                    FormState::Active => {
                        if let Some(form) = &mut self.add_host_form {
                            form.handle_event(&ev);
                        }
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
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        let is_ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);

        if is_ctrl_pressed {
            let action = self.on_key_press_ctrl(key);
            if action != AppKeyAction::Continue {
                return Ok(action);
            }
        }

        match key.code {
            Esc => return Ok(AppKeyAction::Stop),
            Down => self.next(),
            Up => self.previous(),
            Home => self.table_state.select(Some(0)),
            End => self.table_state.select(Some(self.hosts.len() - 1)),
            PageDown => {
                let i = self.table_state.selected().unwrap_or(0);
                let target = min(i.saturating_add(PAGE_SIZE), self.hosts.len() - 1);

                self.table_state.select(Some(target));
            }
            PageUp => {
                let i = self.table_state.selected().unwrap_or(0);
                let target = max(i.saturating_sub(PAGE_SIZE), 0);

                self.table_state.select(Some(target));
            }
            Enter => {
                let selected = self.table_state.selected().unwrap_or(0);
                if selected >= self.hosts.len() {
                    return Ok(AppKeyAction::Ok);
                }

                let host: &ssh::Host = &self.hosts[selected];

                if let Err(e) = safe_restore_terminal(terminal) {
                    // Even if restore fails, we should try to continue
                    eprintln!("Warning: Failed to restore terminal: {}", e);
                }

                if let Some(template) = &self.config.command_template_on_session_start {
                    host.run_command_template(template)?;
                }

                host.run_command_template(&self.config.command_template)?;

                if let Some(template) = &self.config.command_template_on_session_end {
                    host.run_command_template(template)?;
                }

                if let Err(e) = safe_setup_terminal(terminal) {
                    // If we can't restore the terminal, we should exit
                    eprintln!("Fatal error: Failed to setup terminal: {}", e);
                    return Err(e);
                }

                if self.config.exit_after_ssh_session_ends {
                    return Ok(AppKeyAction::Stop);
                }
            }
            _ => return Ok(AppKeyAction::Continue),
        }

        Ok(AppKeyAction::Ok)
    }

    fn on_key_press_ctrl(&mut self, key: KeyEvent) -> AppKeyAction {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Char('c') => AppKeyAction::Stop,
            Char('j' | 'n') => {
                // Check if Ctrl+N is pressed to open the add host form
                if key.code == Char('n') {
                    self.open_add_host_form();
                    return AppKeyAction::Ok;
                }
                
                self.next();
                AppKeyAction::Ok
            }
            Char('k' | 'p') => {
                self.previous();
                AppKeyAction::Ok
            }
            _ => AppKeyAction::Continue,
        }
    }
    
    fn on_form_key_press(&mut self, key: KeyEvent) -> Result<AppKeyAction> {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Esc => Ok(AppKeyAction::Stop),
            Enter => {
                if let Some(form) = &self.add_host_form {
                    if form.is_valid() {
                        // Save the host
                        match self.save_new_host() {
                            Ok(()) => {
                                self.feedback_message = Some("Host added successfully!".to_string());
                                self.is_feedback_error = false;
                                self.form_state = FormState::Hidden;
                                self.add_host_form = None;
                                
                                // Reload the hosts
                                self.reload_hosts()?;
                                
                                return Ok(AppKeyAction::Ok);
                            }
                            Err(e) => {
                                self.feedback_message = Some(format!("Error: {}", e));
                                self.is_feedback_error = true;
                                return Ok(AppKeyAction::Ok);
                            }
                        }
                    }
                    
                    self.feedback_message = Some("Please fill out required fields".to_string());
                    self.is_feedback_error = true;
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
    }
    
    fn save_new_host(&self) -> Result<()> {
        if let Some(form) = &self.add_host_form {
            let config_path = shellexpand::tilde(&self.config.config_paths[1]).to_string();
            form.save_to_config(&config_path)
        } else {
            Err(anyhow::anyhow!("Form is not initialized"))
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