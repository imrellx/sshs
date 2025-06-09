use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Padding, Paragraph, Row, Table},
    text::{Line, Span, Text},
    layout::Margin,
};
use style::palette::tailwind;

use super::app::{
    App, CURSOR_HORIZONTAL_PADDING, CURSOR_VERTICAL_OFFSET, FOOTER_HEIGHT,
    SEARCH_BAR_HEIGHT, SEARCHBAR_HORIZONTAL_PADDING, TABLE_HEADER_HEIGHT,
    TABLE_MIN_HEIGHT, FocusState,
};
use super::form::FormState;

/// Render the UI
pub fn ui(f: &mut Frame, app: &mut App) {
    match app.form_state {
        FormState::Hidden => render_main_ui(f, app),
        FormState::Active => render_form_ui(f, app),
        FormState::Confirming => render_confirmation_ui(f, app),
    }
}

/// Render the main UI
fn render_main_ui(f: &mut Frame, app: &mut App) {
    // If we have active sessions, show tab-based UI
    if !app.tab_manager.is_empty() {
        render_tabbed_ui(f, app);
    } else {
        // Show host selection UI
        render_host_selection_ui(f, app);
    }
}

/// Render the host selection UI (when no tabs are open)
fn render_host_selection_ui(f: &mut Frame, app: &mut App) {
    let rects = Layout::vertical([
        Constraint::Length(SEARCH_BAR_HEIGHT),
        Constraint::Min(TABLE_MIN_HEIGHT),
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .split(f.area());

    render_searchbar(f, app, rects[0]);
    render_table(f, app, rects[1]);
    render_footer_with_mode(f, app, rects[2]);

    // Show feedback message if present
    if let Some(message) = &app.feedback_message {
        render_feedback(f, message, app.is_feedback_error);
    }

    // Show cursor only in search mode
    if matches!(app.focus_state, FocusState::Search) {
        let mut cursor_position = rects[0].as_position();
        cursor_position.x += u16::try_from(app.search.cursor()).unwrap_or_default() + CURSOR_HORIZONTAL_PADDING;
        cursor_position.y += CURSOR_VERTICAL_OFFSET;
        f.set_cursor_position(cursor_position);
    }
}

/// Render the tabbed UI (when sessions are active)
fn render_tabbed_ui(f: &mut Frame, app: &mut App) {
    let main_rects = Layout::vertical([
        Constraint::Length(3), // Tab bar height
        Constraint::Min(0),    // Session content
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .split(f.area());

    // Render tab bar
    let tabs_widget = app.tab_manager.render_tab_bar(main_rects[0]);
    f.render_widget(tabs_widget, main_rects[0]);

    // Render active session content or host selection
    match app.focus_state {
        FocusState::Session => {
            // Show active SSH session
            render_active_session(f, app, main_rects[1]);
        }
        FocusState::Normal | FocusState::Search => {
            // Show host selection overlay for creating new sessions
            render_host_selection_overlay(f, app, main_rects[1]);
        }
        FocusState::SessionManager => {
            // Show session manager overlay
            render_session_manager_overlay(f, app, main_rects[1]);
        }
    }

    // Render footer
    render_footer_with_tabs(f, app, main_rects[2]);

    // Show feedback message if present
    if let Some(message) = &app.feedback_message {
        render_feedback(f, message, app.is_feedback_error);
    }
}

/// Render the form UI
#[allow(clippy::too_many_lines)]
fn render_form_ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    
    // Create a centered box for the form with additional space
    let form_width = 60;
    let form_height = 14; // Base height for the form
    let total_height = form_height + 2; // Add space for help text and field hints
    let horizontal_margin = (area.width.saturating_sub(form_width)) / 2;
    let vertical_margin = (area.height.saturating_sub(total_height)) / 2;
    
    let form_area = Rect::new(
        horizontal_margin,
        vertical_margin,
        form_width,
        form_height,
    );
    
    // Create a block for the form with styled title
    let title = if app.is_edit_mode {
        Line::from(vec![
            Span::styled("Edit SSH Host ", Style::new().fg(app.palette.c400)),
            Span::styled("(Ctrl+E)", Style::new().fg(app.palette.c300).add_modifier(Modifier::ITALIC)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Add New SSH Host ", Style::new().fg(app.palette.c400)),
            Span::styled("(Ctrl+N)", Style::new().fg(app.palette.c300).add_modifier(Modifier::ITALIC)),
        ])
    };
    
    let form_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(app.palette.c400))
        .border_type(BorderType::Rounded);
    
    // Clear the entire form area to prevent artifacts
    f.render_widget(Clear, form_area);
    f.render_widget(form_block, form_area);
    
    // Create inner area for form fields with proper margins
    let inner_area = form_area.inner(Margin::new(2, 1));
    
    // Split the inner area into form fields with spacing between fields
    let chunks = Layout::vertical([
        Constraint::Length(3), // Host name
        Constraint::Length(3), // Hostname/IP
        Constraint::Length(3), // Username
        Constraint::Length(3), // Port
    ])
    .split(inner_area);
    
    if let Some(form) = &app.add_host_form {
        // Render host name field
        let host_name_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(
                if form.active_field == 0 {
                    app.palette.c500
                } else {
                    app.palette.c300
                },
            ))
            .title("Host Name (required)");
        
        let host_name_area = chunks[0];
        f.render_widget(host_name_block, host_name_area);
        
        // Render the actual text content inside the block
        let host_name_inner = host_name_area.inner(Margin::new(1, 1));
        let host_name_text = Paragraph::new(form.host_name.value())
            .style(Style::default().fg(Color::White));
        f.render_widget(Clear, host_name_inner); // Clear the inner area first
        f.render_widget(host_name_text, host_name_inner);
        
        // Render hostname field
        let ip_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(
                if form.active_field == 1 {
                    app.palette.c500
                } else {
                    app.palette.c300
                },
            ))
            .title("Hostname/IP (required)");
        
        let ip_area = chunks[1];
        f.render_widget(ip_block, ip_area);
        
        // Render the actual text content inside the block
        let ip_inner = ip_area.inner(Margin::new(1, 1));
        let ip_text = Paragraph::new(form.hostname.value())
            .style(Style::default().fg(Color::White));
        f.render_widget(Clear, ip_inner); // Clear the inner area first
        f.render_widget(ip_text, ip_inner);
        
        // Render username field
        let username_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(
                if form.active_field == 2 {
                    app.palette.c500
                } else {
                    app.palette.c300
                },
            ))
            .title("Username (optional)");
        
        let username_area = chunks[2];
        f.render_widget(username_block, username_area);
        
        // Render the actual text content inside the block
        let username_inner = username_area.inner(Margin::new(1, 1));
        let username_text = Paragraph::new(form.username.value())
            .style(Style::default().fg(Color::White));
        f.render_widget(Clear, username_inner); // Clear the inner area first
        f.render_widget(username_text, username_inner);
        
        // Render port field
        let port_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(
                if form.active_field == 3 {
                    app.palette.c500
                } else {
                    app.palette.c300
                },
            ))
            .title("Port (optional, numbers only)");
        
        let port_area = chunks[3];
        f.render_widget(port_block, port_area);
        
        // Render the actual text content inside the block
        let port_inner = port_area.inner(Margin::new(1, 1));
        let port_text = Paragraph::new(form.port.value())
            .style(Style::default().fg(Color::White));
        f.render_widget(Clear, port_inner); // Clear the inner area first
        f.render_widget(port_text, port_inner);
        
        // Position cursor in active field
        let active_inner = match form.active_field {
            1 => chunks[1].inner(Margin::new(1, 1)),
            2 => chunks[2].inner(Margin::new(1, 1)),
            3 => chunks[3].inner(Margin::new(1, 1)),
            _ => chunks[0].inner(Margin::new(1, 1)),
        };
        
        // Set cursor position with proper offset
        let mut cursor_position = active_inner.as_position();
        cursor_position.x += u16::try_from(form.active_input().cursor()).unwrap_or_default();
        
        // Show cursor explicitly
        f.set_cursor_position(cursor_position);
    }
    
    // Render keyboard shortcut hints
    let shortcuts = [
        ("Tab", "Next field"),
        ("Shift+Tab", "Previous field"),
        ("Enter", if app.is_edit_mode { "Update" } else { "Save" }),
        ("Esc", "Cancel"),
    ];
    
    // Create a styled help text with highlighted keys
    let mut help_spans = Vec::new();
    for (i, (key, action)) in shortcuts.iter().enumerate() {
        // Add separator between items
        if i > 0 {
            help_spans.push(Span::styled(" | ", Style::new().fg(app.palette.c300)));
        }
        
        // Add key with highlight
        help_spans.push(Span::styled(
            key.to_string(),
            Style::new().fg(app.palette.c500).add_modifier(Modifier::BOLD),
        ));
        
        // Add description
        help_spans.push(Span::styled(
            format!(" {}", action),
            Style::new().fg(app.palette.c300),
        ));
    }
    
    let help_line = Line::from(help_spans);
    let help_paragraph = Paragraph::new(help_line)
        .alignment(Alignment::Center);
    
    let help_area = Rect::new(
        horizontal_margin,
        vertical_margin + form_height,
        form_width,
        1,
    );
    
    f.render_widget(help_paragraph, help_area);
    
    // Add field-specific hints
    if let Some(form) = &app.add_host_form {
        let hint_text = match form.active_field {
            0 => "Host name used to identify this connection (required)",
            1 => "IP address or domain name to connect to (required)",
            2 => "SSH username (optional, will use system default if empty)",
            3 => "SSH port (optional, defaults to 22 if empty)",
            _ => "",
        };
        
        let hint_paragraph = Paragraph::new(Line::from(hint_text))
            .alignment(Alignment::Center)
            .style(Style::new().fg(app.palette.c200));
        
        let hint_area = Rect::new(
            horizontal_margin,
            vertical_margin + form_height + 1,
            form_width,
            1,
        );
        
        f.render_widget(hint_paragraph, hint_area);
    }
    
    // Show feedback message if present
    if let Some(message) = &app.feedback_message {
        render_feedback(f, message, app.is_feedback_error);
    }
}

/// Render a confirmation dialog
fn render_confirmation_ui(f: &mut Frame, app: &mut App) {
    // First render the form UI in the background
    render_form_ui(f, app);
    
    let area = f.area();
    
    // Create a centered box for the confirmation dialog
    let message = app.confirm_message.as_deref().unwrap_or("Confirm?");
    let dialog_width = 50.max(u16::try_from(message.len()).unwrap_or(50) + 4);
    let dialog_height = 7; // Increased height for buttons
    let horizontal_margin = (area.width.saturating_sub(dialog_width)) / 2;
    let vertical_margin = (area.height.saturating_sub(dialog_height)) / 2;
    
    let dialog_area = Rect::new(
        horizontal_margin,
        vertical_margin,
        dialog_width,
        dialog_height,
    );
    
    // Clear the area first
    f.render_widget(Clear, dialog_area);
    
    // Create a block for the dialog
    let dialog_block = Block::default()
        .title("Confirmation Required")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(tailwind::ORANGE.c500))
        .border_type(BorderType::Rounded);
    
    f.render_widget(dialog_block, dialog_area);
    
    // Split the inner area into message and buttons
    let inner_area = dialog_area.inner(Margin::new(2, 1));
    let chunks = Layout::vertical([
        Constraint::Length(1), // Message
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Buttons
    ])
    .split(inner_area);
    
    // Render message
    let message_paragraph = Paragraph::new(Line::from(message))
        .alignment(Alignment::Center)
        .style(Style::new().fg(Color::White));
    
    f.render_widget(message_paragraph, chunks[0]);
    
    // Render buttons with styled keyboard shortcuts
    let action_text = app.confirm_action.as_deref().unwrap_or("Yes");
    
    let button_spans = vec![
        // Yes button
        Span::styled("(", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled("Y", Style::new().fg(tailwind::GREEN.c500).add_modifier(Modifier::BOLD)),
        Span::styled(") ", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled(action_text, Style::new().fg(tailwind::GREEN.c500)),
        
        // Separator
        Span::styled(" | ", Style::new().fg(tailwind::BLUE.c400)),
        
        // No button
        Span::styled("(", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled("N", Style::new().fg(tailwind::RED.c500).add_modifier(Modifier::BOLD)),
        Span::styled(") ", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled("Cancel", Style::new().fg(tailwind::RED.c500)),
    ];
    
    let buttons_line = Line::from(button_spans);
    let buttons_paragraph = Paragraph::new(buttons_line)
        .alignment(Alignment::Center);
    
    f.render_widget(buttons_paragraph, chunks[2]);
}

/// Render a feedback message
fn render_feedback(f: &mut Frame, message: &str, is_error: bool) {
    let area = f.area();
    
    // Create a centered box for the message
    let message_width = 40.max(u16::try_from(message.len()).unwrap_or(40) + 4);
    let message_height = 3;
    let horizontal_margin = (area.width.saturating_sub(message_width)) / 2;
    let vertical_margin = (area.height.saturating_sub(message_height)) / 2;
    
    let message_area = Rect::new(
        horizontal_margin,
        vertical_margin - 10, // Position above the center
        message_width,
        message_height,
    );
    
    // Clear the area first
    f.render_widget(Clear, message_area);
    
    // Create a block for the message
    let message_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if is_error {
            tailwind::RED.c500
        } else {
            tailwind::GREEN.c500
        }))
        .border_type(BorderType::Rounded);
    
    let message_paragraph = Paragraph::new(Line::from(message))
        .block(message_block)
        .alignment(Alignment::Center)
        .style(Style::new().fg(if is_error {
            tailwind::RED.c500
        } else {
            tailwind::GREEN.c500
        }));
    
    f.render_widget(message_paragraph, message_area);
}

/// Render the search bar
pub fn render_searchbar(f: &mut Frame, app: &mut App, area: Rect) {
    // Use different styling based on focus state
    let border_style = if matches!(app.focus_state, crate::ui::app::FocusState::Search) {
        Style::new().fg(app.palette.c500) // Brighter when focused
    } else {
        Style::new().fg(app.palette.c300) // Dimmer when not focused
    };
    
    let info_footer = Paragraph::new(Line::from(app.search.value())).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(SEARCHBAR_HORIZONTAL_PADDING)),
    );
    f.render_widget(info_footer, area);
}

/// Render the table
pub fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default().fg(tailwind::CYAN.c500);
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);

    let mut header_names = vec!["Name", "Aliases", "User", "Destination", "Port"];
    if app.config.show_proxy_command {
        header_names.push("Proxy");
    }

    let header = header_names
        .iter()
        .copied()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(TABLE_HEADER_HEIGHT);

    let rows = app.hosts.iter().map(|host| {
        let mut content = vec![
            host.name.clone(),
            host.aliases.clone(),
            host.user.clone().unwrap_or_default(),
            host.destination.clone(),
            host.port.clone().unwrap_or_default(),
        ];
        if app.config.show_proxy_command {
            content.push(host.proxy_command.clone().unwrap_or_default());
        }

        content
            .iter()
            .map(|content| Cell::from(Text::from(content.to_string())))
            .collect::<Row>()
    });

    let bar = " █ ";
    let t = Table::new(rows, app.table_columns_constraints.clone())
        .header(header)
        .row_highlight_style(selected_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .highlight_spacing(HighlightSpacing::Always)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.palette.c400))
                .border_type(BorderType::Rounded),
        );

    f.render_stateful_widget(t, area, &mut app.table_state);
}

/// Render the footer with mode indicator
pub fn render_footer_with_mode(f: &mut Frame, app: &mut App, area: Rect) {
    let (mode_text, shortcuts_text) = match app.focus_state {
        crate::ui::app::FocusState::Normal => {
            let mode = "-- NORMAL --";
            let shortcuts = "(j/k/↑/↓) navigate | (/) search | (enter) connect | (n) new | (e) edit | (d) delete | (q) quit";
            (mode, shortcuts)
        }
        crate::ui::app::FocusState::Search => {
            let mode = "-- SEARCH --";
            let shortcuts = "(type to search) | (enter) keep filter | (esc) clear & exit | (Ctrl+F) also opens search";
            (mode, shortcuts)
        }
        crate::ui::app::FocusState::Session => {
            let mode = "-- SESSION --";
            let shortcuts = "(esc) back to hosts | (gt) next tab | (gT) prev tab | (Ctrl+W) close | (q) quit";
            (mode, shortcuts)
        }
        crate::ui::app::FocusState::SessionManager => {
            let mode = "-- SESSION MANAGER --";
            let shortcuts = "(esc/q) close manager | (enter) switch | (d) disconnect | (n) new session";
            (mode, shortcuts)
        }
    };
    
    // Create the footer text with mode indicator and shortcuts
    let footer_line = Line::from(vec![
        Span::styled(mode_text, Style::new().fg(app.palette.c500).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(shortcuts_text, Style::new().fg(app.palette.c300)),
    ]);
    
    let info_footer = Paragraph::new(footer_line).centered().block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.palette.c400))
            .border_type(BorderType::Rounded),
    );
    f.render_widget(info_footer, area);
}

/// Render the footer (legacy function - kept for compatibility)
pub fn render_footer(f: &mut Frame, app: &mut App, area: Rect) {
    render_footer_with_mode(f, app, area);
}

/// Render the active SSH session
fn render_active_session(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(session) = app.tab_manager.get_active_session() {
        // Create a terminal view showing the session output
        let terminal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.palette.c400))
            .title(format!(" {} ", session.display_name))
            .title_style(Style::default().fg(app.palette.c500).add_modifier(Modifier::BOLD));

        // For now, show a placeholder for the terminal content
        let terminal_content = vec![
            Line::from(""),
            Line::from(Span::styled("SSH Terminal Session", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Host: ", Style::default().fg(app.palette.c300)),
                Span::styled(&session.host.destination, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("User: ", Style::default().fg(app.palette.c300)),
                Span::styled(session.host.user.as_deref().unwrap_or("default"), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(app.palette.c300)),
                Span::styled(
                    format!("{:?}", session.state),
                    match session.state {
                        super::session::ConnectionState::Connected => Style::default().fg(Color::Green),
                        super::session::ConnectionState::Connecting => Style::default().fg(Color::Yellow),
                        super::session::ConnectionState::Disconnected => Style::default().fg(Color::Red),
                        super::session::ConnectionState::Error(_) => Style::default().fg(Color::Red),
                        super::session::ConnectionState::Reconnecting => Style::default().fg(Color::Yellow),
                    }
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled("Terminal output will appear here...", Style::default().fg(app.palette.c300))),
            Line::from(""),
            Line::from(Span::styled("TODO: Implement VT100 terminal rendering", Style::default().fg(Color::Yellow))),
        ];

        let terminal_paragraph = Paragraph::new(terminal_content)
            .block(terminal_block)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(terminal_paragraph, area);
    } else {
        // No active session
        let placeholder = Paragraph::new("No active session")
            .block(Block::default().borders(Borders::ALL).title(" Session "))
            .style(Style::default().fg(app.palette.c300));
        f.render_widget(placeholder, area);
    }
}

/// Render host selection overlay for creating new sessions
fn render_host_selection_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    // Create a smaller area for the host selection
    let overlay_area = Layout::vertical([
        Constraint::Length(3), // Search bar
        Constraint::Min(0),    // Host table
        Constraint::Length(2), // Instructions
    ])
    .split(area);

    // Render search bar
    render_searchbar(f, app, overlay_area[0]);
    
    // Render host table
    render_table(f, app, overlay_area[1]);
    
    // Render instructions
    let instructions = Paragraph::new("Press Enter to connect to selected host in new tab, or use Ctrl+T")
        .style(Style::default().fg(app.palette.c300))
        .alignment(Alignment::Center);
    f.render_widget(instructions, overlay_area[2]);

    // Show cursor in search mode
    if matches!(app.focus_state, FocusState::Search) {
        let mut cursor_position = overlay_area[0].as_position();
        cursor_position.x += u16::try_from(app.search.cursor()).unwrap_or_default() + CURSOR_HORIZONTAL_PADDING;
        cursor_position.y += CURSOR_VERTICAL_OFFSET;
        f.set_cursor_position(cursor_position);
    }
}

/// Render session manager overlay
fn render_session_manager_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    // Create a centered dialog for the session manager
    let dialog_area = {
        let margin_horizontal = area.width / 6;
        let margin_vertical = area.height / 6;
        Rect {
            x: area.x + margin_horizontal,
            y: area.y + margin_vertical,
            width: area.width.saturating_sub(margin_horizontal * 2),
            height: area.height.saturating_sub(margin_vertical * 2),
        }
    };

    // Clear the background
    f.render_widget(Clear, dialog_area);

    // Get session statistics
    let session_stats = app.tab_manager.get_session_stats();
    
    // Create table rows
    let rows: Vec<Row> = session_stats
        .iter()
        .map(|stat| {
            let status_symbol = match stat.state {
                super::session::ConnectionState::Connected => "●",
                super::session::ConnectionState::Connecting => "○",
                super::session::ConnectionState::Disconnected => "✗",
                super::session::ConnectionState::Error(_) => "!",
                super::session::ConnectionState::Reconnecting => "↻",
            };
            
            let status_color = match stat.state {
                super::session::ConnectionState::Connected => Color::Green,
                super::session::ConnectionState::Connecting => Color::Yellow,
                super::session::ConnectionState::Disconnected => Color::Red,
                super::session::ConnectionState::Error(_) => Color::Red,
                super::session::ConnectionState::Reconnecting => Color::Yellow,
            };

            let activity_bars = "●●●○○"; // Placeholder for activity indicator
            let uptime = format!("{}m", stat.stats.uptime.as_secs() / 60);
            
            Row::new([
                Cell::from(stat.tab_number.to_string()).style(Style::default().fg(app.palette.c300)),
                Cell::from(stat.name.clone()).style(if stat.is_active {
                    Style::default().fg(app.palette.c100).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
                Cell::from(stat.host.clone()).style(Style::default().fg(app.palette.c300)),
                Cell::from(status_symbol).style(Style::default().fg(status_color)),
                Cell::from(activity_bars).style(Style::default().fg(app.palette.c400)),
                Cell::from(uptime).style(Style::default().fg(app.palette.c300)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),  // Tab
            Constraint::Min(15),    // Name
            Constraint::Min(20),    // Host
            Constraint::Length(6),  // Status
            Constraint::Length(8),  // Activity
            Constraint::Length(8),  // Uptime
        ]
    )
    .header(
        Row::new(["Tab", "Name", "Host", "Status", "Activity", "Uptime"])
            .style(Style::default().fg(app.palette.c500).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.palette.c400))
            .title(" Session Manager ")
            .title_style(Style::default().fg(app.palette.c500).add_modifier(Modifier::BOLD))
    );

    f.render_widget(table, dialog_area);

    // Render instructions at the bottom
    let instructions_area = Rect {
        x: dialog_area.x,
        y: dialog_area.y + dialog_area.height - 3,
        width: dialog_area.width,
        height: 2,
    };

    let instructions = Paragraph::new("Commands: Enter=Switch, D=Disconnect, R=Rename, N=New Session, X=Close, Q=Quit Manager")
        .style(Style::default().fg(app.palette.c300))
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(instructions, instructions_area);
}

/// Render footer with tab information
fn render_footer_with_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let session_count = app.tab_manager.session_count();
    let active_tab = app.tab_manager.get_active_tab_index() + 1;
    
    let footer_text = if session_count > 0 {
        format!(
            "Tab {}/{} | (gt) next tab | (gT) prev tab | (Ctrl+T) new | (Ctrl+W) close | (Ctrl+Shift+S) manager | (q) quit",
            active_tab, session_count
        )
    } else {
        "(Enter) connect | (Ctrl+T) new tab | (Ctrl+N) new host | (q) quit".to_string()
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(app.palette.c400))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(app.palette.c700))
        );

    f.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::form::AddHostForm;
    use crate::ui::app::{App, AppConfig, FocusState};
    use crate::searchable::Searchable;
    use tui_input::Input;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::TableState;

    /// Test helper to create a minimal app for rendering tests
    fn create_test_app() -> App {
        let config = AppConfig {
            config_paths: vec!["/etc/ssh/ssh_config".to_string(), "~/.ssh/config".to_string()],
            search_filter: None,
            sort_by_name: true,
            show_proxy_command: false,
            command_template: "ssh {destination}".to_string(),
            command_template_on_session_start: None,
            command_template_on_session_end: None,
            exit_after_ssh_session_ends: false,
        };

        let session_config = crate::ui::session::SessionConfig::default();
        let tab_manager = crate::ui::tabs::TabManager::new(session_config, tailwind::BLUE);

        App {
            config,
            search: Input::default(),
            table_state: TableState::default(),
            hosts: Searchable::new(Vec::new(), "", |_, _| true),
            table_columns_constraints: vec![
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
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
    fn test_form_ui_rendering() {
        // Create a test backend with a fixed size
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        
        // Create a test app with a form
        let mut app = create_test_app();
        app.form_state = FormState::Active;
        app.add_host_form = Some(AddHostForm::new());
        
        // Draw the UI
        terminal.draw(|f| render_form_ui(f, &mut app)).unwrap();
        
        // Get the buffer after rendering
        let buffer = terminal.backend().buffer().clone();
        
        // Verify form title is rendered
        assert!(buffer_contains_text(&buffer, "Add New SSH Host"));
        
        // Verify form field labels are rendered
        assert!(buffer_contains_text(&buffer, "Host Name"));
        assert!(buffer_contains_text(&buffer, "Hostname/IP"));
        assert!(buffer_contains_text(&buffer, "Username"));
        assert!(buffer_contains_text(&buffer, "Port"));
        
        // Verify help text is rendered
        assert!(buffer_contains_text(&buffer, "Tab"));
        assert!(buffer_contains_text(&buffer, "Next field"));
        assert!(buffer_contains_text(&buffer, "Enter"));
        assert!(buffer_contains_text(&buffer, "Save"));
        
        // Test with filled form fields
        let mut form = AddHostForm::new();
        form.host_name = Input::from("test-host".to_string());
        form.hostname = Input::from("example.com".to_string());
        form.username = Input::from("user".to_string());
        form.port = Input::from("22".to_string());
        
        app.add_host_form = Some(form);
        
        // Draw the UI again
        terminal.draw(|f| render_form_ui(f, &mut app)).unwrap();
        
        // Get the buffer after rendering
        let buffer = terminal.backend().buffer().clone();
        
        // Verify form field values are rendered
        assert!(buffer_contains_text(&buffer, "test-host"));
        assert!(buffer_contains_text(&buffer, "example.com"));
        assert!(buffer_contains_text(&buffer, "user"));
        assert!(buffer_contains_text(&buffer, "22"));
    }
    
    #[test]
    fn test_confirmation_dialog_rendering() {
        // Create a test backend with a fixed size
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        
        // Create a test app with a confirmation dialog
        let mut app = create_test_app();
        app.form_state = FormState::Confirming;
        app.add_host_form = Some(AddHostForm::new());
        app.confirm_message = Some("Host 'test-host' already exists. Overwrite?".to_string());
        app.confirm_action = Some("Overwrite".to_string());
        
        // Draw the UI
        terminal.draw(|f| render_confirmation_ui(f, &mut app)).unwrap();
        
        // Get the buffer after rendering
        let buffer = terminal.backend().buffer().clone();
        
        // Verify confirmation dialog elements are rendered
        assert!(buffer_contains_text(&buffer, "Confirmation Required"));
        assert!(buffer_contains_text(&buffer, "Host 'test-host' already exists"));
        assert!(buffer_contains_text(&buffer, "Overwrite"));
        assert!(buffer_contains_text(&buffer, "Cancel"));
        assert!(buffer_contains_text(&buffer, "(Y)"));
        assert!(buffer_contains_text(&buffer, "(N)"));
    }
    
    #[test]
    fn test_feedback_message_rendering() {
        // Create a test backend with a fixed size
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        
        // Create a test app with a success message
        let mut app = create_test_app();
        app.form_state = FormState::Active;
        app.add_host_form = Some(AddHostForm::new());
        app.feedback_message = Some("Host added successfully!".to_string());
        app.is_feedback_error = false;
        
        // Draw the UI
        terminal.draw(|f| render_form_ui(f, &mut app)).unwrap();
        
        // Get the buffer after rendering
        let buffer = terminal.backend().buffer().clone();
        
        // Verify success message is rendered
        assert!(buffer_contains_text(&buffer, "Host added successfully!"));
        
        // Create a test app with an error message
        let mut app = create_test_app();
        app.form_state = FormState::Active;
        app.add_host_form = Some(AddHostForm::new());
        app.feedback_message = Some("Invalid hostname format".to_string());
        app.is_feedback_error = true;
        
        // Draw the UI again
        terminal.draw(|f| render_form_ui(f, &mut app)).unwrap();
        
        // Get the buffer after rendering
        let buffer = terminal.backend().buffer().clone();
        
        // Verify error message is rendered
        assert!(buffer_contains_text(&buffer, "Invalid hostname format"));
    }
    
    #[test]
    fn test_form_field_navigation() {
        // Create a test backend with a fixed size
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        
        // Create a test app with a form
        let mut app = create_test_app();
        app.form_state = FormState::Active;
        
        // Test with different active fields
        for field_idx in 0..4 {
            let mut form = AddHostForm::new();
            form.active_field = field_idx;
            app.add_host_form = Some(form);
            
            // Draw the UI
            terminal.draw(|f| render_form_ui(f, &mut app)).unwrap();
            
            // Get the buffer after rendering
            let buffer = terminal.backend().buffer().clone();
            
            // Verify field-specific hint is rendered
            match field_idx {
                0 => assert!(buffer_contains_text(&buffer, "Host name used to identify")),
                1 => assert!(buffer_contains_text(&buffer, "IP address or domain name")),
                2 => assert!(buffer_contains_text(&buffer, "SSH username")),
                3 => assert!(buffer_contains_text(&buffer, "SSH port")),
                _ => {}
            }
        }
    }

    /// Helper function to check if a buffer contains specific text
    fn buffer_contains_text(buffer: &Buffer, text: &str) -> bool {
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<Vec<_>>()
            .join("");
        
        content.contains(text)
    }
}