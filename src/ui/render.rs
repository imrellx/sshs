use ratatui::{
    layout::Margin,
    prelude::*,
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Padding, Paragraph, Row, Table,
    },
};
use style::palette::tailwind;

use super::app::{
    App, CURSOR_HORIZONTAL_PADDING, CURSOR_VERTICAL_OFFSET, FOOTER_HEIGHT,
    SEARCHBAR_HORIZONTAL_PADDING, SEARCH_BAR_HEIGHT, TABLE_HEADER_HEIGHT, TABLE_MIN_HEIGHT,
};
use super::form::FormState;

/// Render the UI
pub fn ui(f: &mut Frame, app: &mut App) {
    match app.form_state {
        FormState::Hidden => {
            render_main_ui(f, app);
            // Render session manager overlay if active
            if app.focus_state == super::app::FocusState::SessionManager {
                render_session_manager_overlay(f, app);
            }
        }
        FormState::Active => render_form_ui(f, app),
        FormState::Confirming => render_confirmation_ui(f, app),
    }
}

/// Render the main UI
fn render_main_ui(f: &mut Frame, app: &mut App) {
    // Create layout based on whether tabs exist
    let rects = if app.tab_manager.has_sessions() {
        Layout::vertical([
            Constraint::Length(1), // Tab bar
            Constraint::Length(SEARCH_BAR_HEIGHT),
            Constraint::Min(TABLE_MIN_HEIGHT),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(f.area())
    } else {
        Layout::vertical([
            Constraint::Length(SEARCH_BAR_HEIGHT),
            Constraint::Min(TABLE_MIN_HEIGHT),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(f.area())
    };

    let mut rect_index = 0;

    // Render tab bar if sessions exist
    if app.tab_manager.has_sessions() {
        render_tab_bar(f, app, rects[rect_index]);
        rect_index += 1;
    }

    render_searchbar(f, app, rects[rect_index]);
    render_table(f, app, rects[rect_index + 1]);
    render_footer_with_mode(f, app, rects[rect_index + 2]);

    // Show feedback message if present
    if let Some(message) = &app.feedback_message {
        render_feedback(f, message, app.is_feedback_error);
    }

    // Show cursor only in search mode
    if matches!(app.focus_state, crate::ui::app::FocusState::Search) {
        let mut cursor_position = rects[0].as_position();
        cursor_position.x +=
            u16::try_from(app.search.cursor()).unwrap_or_default() + CURSOR_HORIZONTAL_PADDING;
        cursor_position.y += CURSOR_VERTICAL_OFFSET;
        f.set_cursor_position(cursor_position);
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

    let form_area = Rect::new(horizontal_margin, vertical_margin, form_width, form_height);

    // Create a block for the form with styled title
    let title = if app.is_edit_mode {
        Line::from(vec![
            Span::styled("Edit SSH Host ", Style::new().fg(app.palette.c400)),
            Span::styled(
                "(Ctrl+E)",
                Style::new()
                    .fg(app.palette.c300)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("Add New SSH Host ", Style::new().fg(app.palette.c400)),
            Span::styled(
                "(Ctrl+N)",
                Style::new()
                    .fg(app.palette.c300)
                    .add_modifier(Modifier::ITALIC),
            ),
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
            .border_style(Style::new().fg(if form.active_field == 0 {
                app.palette.c500
            } else {
                app.palette.c300
            }))
            .title("Host Name (required)");

        let host_name_area = chunks[0];
        f.render_widget(host_name_block, host_name_area);

        // Render the actual text content inside the block
        let host_name_inner = host_name_area.inner(Margin::new(1, 1));
        let host_name_text =
            Paragraph::new(form.host_name.value()).style(Style::default().fg(Color::White));
        f.render_widget(Clear, host_name_inner); // Clear the inner area first
        f.render_widget(host_name_text, host_name_inner);

        // Render hostname field
        let ip_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(if form.active_field == 1 {
                app.palette.c500
            } else {
                app.palette.c300
            }))
            .title("Hostname/IP (required)");

        let ip_area = chunks[1];
        f.render_widget(ip_block, ip_area);

        // Render the actual text content inside the block
        let ip_inner = ip_area.inner(Margin::new(1, 1));
        let ip_text =
            Paragraph::new(form.hostname.value()).style(Style::default().fg(Color::White));
        f.render_widget(Clear, ip_inner); // Clear the inner area first
        f.render_widget(ip_text, ip_inner);

        // Render username field
        let username_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(if form.active_field == 2 {
                app.palette.c500
            } else {
                app.palette.c300
            }))
            .title("Username (optional)");

        let username_area = chunks[2];
        f.render_widget(username_block, username_area);

        // Render the actual text content inside the block
        let username_inner = username_area.inner(Margin::new(1, 1));
        let username_text =
            Paragraph::new(form.username.value()).style(Style::default().fg(Color::White));
        f.render_widget(Clear, username_inner); // Clear the inner area first
        f.render_widget(username_text, username_inner);

        // Render port field
        let port_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(if form.active_field == 3 {
                app.palette.c500
            } else {
                app.palette.c300
            }))
            .title("Port (optional, numbers only)");

        let port_area = chunks[3];
        f.render_widget(port_block, port_area);

        // Render the actual text content inside the block
        let port_inner = port_area.inner(Margin::new(1, 1));
        let port_text = Paragraph::new(form.port.value()).style(Style::default().fg(Color::White));
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
            (*key).to_string(),
            Style::new()
                .fg(app.palette.c500)
                .add_modifier(Modifier::BOLD),
        ));

        // Add description
        help_spans.push(Span::styled(
            format!(" {action}"),
            Style::new().fg(app.palette.c300),
        ));
    }

    let help_line = Line::from(help_spans);
    let help_paragraph = Paragraph::new(help_line).alignment(Alignment::Center);

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
        Span::styled(
            "Y",
            Style::new()
                .fg(tailwind::GREEN.c500)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(") ", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled(action_text, Style::new().fg(tailwind::GREEN.c500)),
        // Separator
        Span::styled(" | ", Style::new().fg(tailwind::BLUE.c400)),
        // No button
        Span::styled("(", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled(
            "N",
            Style::new()
                .fg(tailwind::RED.c500)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(") ", Style::new().fg(tailwind::BLUE.c400)),
        Span::styled("Cancel", Style::new().fg(tailwind::RED.c500)),
    ];

    let buttons_line = Line::from(button_spans);
    let buttons_paragraph = Paragraph::new(buttons_line).alignment(Alignment::Center);

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

/// Render the tab bar
pub fn render_tab_bar(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.tab_manager.has_sessions() {
        return;
    }

    let sessions = app.tab_manager.sessions();
    let current_index = app.tab_manager.current_session_index();

    // Create tab spans
    let mut tab_spans = Vec::new();

    for (index, session) in sessions.iter().enumerate() {
        let tab_text = session.tab_display_name();

        if index == current_index {
            // Current tab - highlighted
            tab_spans.push(Span::styled(
                format!("▶{tab_text}"),
                Style::default()
                    .fg(Color::White)
                    .bg(app.palette.c600)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            // Inactive tab
            tab_spans.push(Span::styled(
                tab_text,
                Style::default().fg(app.palette.c400).bg(app.palette.c950),
            ));
        }
    }

    // Add instructions for new users
    if app.tab_manager.session_count() < 3 {
        tab_spans.push(Span::styled(
            " | Ctrl+N: New | Ctrl+1/2/3: Switch",
            Style::default().fg(app.palette.c300),
        ));
    }

    let tab_line = Line::from(tab_spans);
    let tab_paragraph = Paragraph::new(tab_line);

    f.render_widget(tab_paragraph, area);
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
        crate::ui::app::FocusState::SessionManager => {
            let mode = "-- SESSION MANAGER --";
            let shortcuts = "(↑/↓) navigate | (enter) switch | (r) rename | (d) disconnect | (x) close | (q/esc) quit";
            (mode, shortcuts)
        }
        crate::ui::app::FocusState::RenameSession => {
            let mode = "-- RENAME SESSION --";
            let shortcuts = "(type new name) | (enter) confirm | (esc) cancel";
            (mode, shortcuts)
        }
    };

    // Create the footer text with mode indicator and shortcuts
    let footer_line = Line::from(vec![
        Span::styled(
            mode_text,
            Style::new()
                .fg(app.palette.c500)
                .add_modifier(Modifier::BOLD),
        ),
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

/// Render the session manager overlay
fn render_session_manager_overlay(f: &mut Frame, app: &mut App) {
    // Calculate overlay dimensions (centered, 60% of screen width, 70% of height)
    let area = f.area();
    let overlay_width = (area.width * 60) / 100;
    let overlay_height = (area.height * 70) / 100;
    let horizontal_margin = (area.width.saturating_sub(overlay_width)) / 2;
    let vertical_margin = (area.height.saturating_sub(overlay_height)) / 2;
    let overlay_area = Rect::new(
        horizontal_margin,
        vertical_margin,
        overlay_width,
        overlay_height,
    );

    // Clear the overlay area
    f.render_widget(Clear, overlay_area);

    // Create the outer block with title and borders
    let block = Block::default()
        .title(" Session Manager ")
        .title_style(Style::default().fg(tailwind::BLUE.c200).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(tailwind::BLUE.c400))
        .style(Style::default().bg(tailwind::SLATE.c900));

    f.render_widget(block, overlay_area);

    // Calculate inner area for content
    let inner_area = overlay_area.inner(Margin::new(1, 1));

    if !app.tab_manager.has_sessions() {
        // Show empty message
        let empty_text = Paragraph::new("No active sessions")
            .style(Style::default().fg(tailwind::SLATE.c400))
            .alignment(Alignment::Center);
        f.render_widget(empty_text, inner_area);
        return;
    }

    // Split into session list and help area
    let chunks = Layout::vertical([
        Constraint::Min(5),    // Session list
        Constraint::Length(3), // Help text
    ])
    .split(inner_area);

    // Render session list
    render_session_list(f, app, chunks[0]);

    // Render help text
    render_session_manager_help(f, chunks[1]);
}

/// Render the session list table
fn render_session_list(f: &mut Frame, app: &mut App, area: Rect) {
    let sessions = app.tab_manager.sessions();
    let current_session_index = app.tab_manager.current_session_index();

    // Create table rows
    let rows: Vec<Row> = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let session_number = (index + 1).to_string();
            let name = session.host.name.clone();
            let status = match session.status {
                crate::ui::tabs::SessionStatus::Connected => "Connected",
                crate::ui::tabs::SessionStatus::Reconnecting => "Reconnecting",
                crate::ui::tabs::SessionStatus::Disconnected => "Disconnected",
            };

            // Build activity indicator string
            let mut activity = String::new();
            if session.activity.has_new_output {
                activity.push('*');
            }
            if session.activity.has_error {
                activity.push('!');
            }
            if session.activity.has_background_activity {
                activity.push('@');
            }
            if activity.is_empty() {
                activity = "-".to_string();
            }

            // Add current session indicator
            let indicator = if index == current_session_index {
                "▶"
            } else {
                " "
            };

            let row_style = if index == app.session_manager_selection_index {
                // Highlight selected row
                Style::default()
                    .bg(tailwind::BLUE.c800)
                    .fg(tailwind::BLUE.c100)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(indicator),
                Cell::from(session_number),
                Cell::from(name),
                Cell::from(status),
                Cell::from(activity),
            ])
            .style(row_style)
        })
        .collect();

    // Create table headers
    let headers = Row::new(vec!["", "Tab", "Name", "Status", "Activity"])
        .style(Style::default().fg(tailwind::SLATE.c300).bold())
        .bottom_margin(1);

    // Create the table
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),  // Current indicator
            Constraint::Length(4),  // Tab number
            Constraint::Min(15),    // Name (flexible)
            Constraint::Length(12), // Status
            Constraint::Length(8),  // Activity
        ],
    )
    .header(headers)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(tailwind::SLATE.c600))
            .title(" Sessions ")
            .title_style(Style::default().fg(tailwind::SLATE.c300)),
    )
    .row_highlight_style(Style::default().bg(tailwind::BLUE.c800))
    .highlight_spacing(HighlightSpacing::Always);

    f.render_widget(table, area);
}

/// Render the help text for session manager commands
fn render_session_manager_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![
            Span::styled(
                "Commands: ",
                Style::default().fg(tailwind::SLATE.c300).bold(),
            ),
            Span::styled("Enter", Style::default().fg(tailwind::GREEN.c400).bold()),
            Span::raw("=Switch  "),
            Span::styled("R", Style::default().fg(tailwind::BLUE.c400).bold()),
            Span::raw("=Rename  "),
            Span::styled("X", Style::default().fg(tailwind::RED.c400).bold()),
            Span::raw("=Close"),
        ]),
        Line::from(vec![
            Span::raw("         "),
            Span::styled("D", Style::default().fg(tailwind::YELLOW.c400).bold()),
            Span::raw("=Disconnect  "),
            Span::styled("Q/Esc", Style::default().fg(tailwind::SLATE.c400).bold()),
            Span::raw("=Quit Manager"),
        ]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(tailwind::SLATE.c600))
                .title(" Commands ")
                .title_style(Style::default().fg(tailwind::SLATE.c300)),
        )
        .alignment(Alignment::Center);

    f.render_widget(help_paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::searchable::Searchable;
    use crate::ui::app::{App, AppConfig, FocusState};
    use crate::ui::form::AddHostForm;
    use crate::ui::tabs::TabManager;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::widgets::TableState;
    use tui_input::Input;

    /// Test helper to create a minimal app for rendering tests
    fn create_test_app() -> App {
        let config = AppConfig {
            config_paths: vec![
                "/etc/ssh/ssh_config".to_string(),
                "~/.ssh/config".to_string(),
            ],
            search_filter: None,
            sort_by_name: true,
            show_proxy_command: false,
            command_template: "ssh {destination}".to_string(),
            command_template_on_session_start: None,
            command_template_on_session_end: None,
            exit_after_ssh_session_ends: false,
        };

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
            tab_manager: TabManager::new(),
            session_manager_selection_index: 0,
            session_rename_input: None,
            session_rename_index: None,
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
        terminal
            .draw(|f| render_confirmation_ui(f, &mut app))
            .unwrap();

        // Get the buffer after rendering
        let buffer = terminal.backend().buffer().clone();

        // Verify confirmation dialog elements are rendered
        assert!(buffer_contains_text(&buffer, "Confirmation Required"));
        assert!(buffer_contains_text(
            &buffer,
            "Host 'test-host' already exists"
        ));
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
            .collect::<String>();

        content.contains(text)
    }

    #[test]
    fn test_tab_bar_rendering() {
        use crate::ssh::Host;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app();

        // Add some sessions
        let host1 = Host {
            name: "prod-web".to_string(),
            destination: "prod-web.com".to_string(),
            user: None,
            port: None,
            aliases: String::new(),
            proxy_command: None,
        };
        let host2 = Host {
            name: "dev-db".to_string(),
            destination: "dev-db.com".to_string(),
            user: None,
            port: None,
            aliases: String::new(),
            proxy_command: None,
        };

        app.tab_manager.add_session(host1).unwrap();
        app.tab_manager.add_session(host2).unwrap();

        // Switch to first tab
        app.tab_manager.switch_to_session(1);

        // Render the UI
        terminal
            .draw(|f| {
                super::ui(f, &mut app);
            })
            .unwrap();

        // Get the rendered content
        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<String>();

        // Check that tab content is rendered
        assert!(content.contains("[1:prod-web]"), "Should contain first tab");
        assert!(content.contains("[2:dev-db]"), "Should contain second tab");
        assert!(content.contains("▶"), "Should show current tab indicator");
        assert!(content.contains("Ctrl+N"), "Should show instructions");
    }

    #[test]
    fn test_tab_bar_not_rendered_when_no_sessions() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app();

        // Render the UI with no sessions
        terminal
            .draw(|f| {
                super::ui(f, &mut app);
            })
            .unwrap();

        // Get the rendered content
        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<String>();

        // Check that no tab content is rendered
        assert!(
            !content.contains("[1:"),
            "Should not contain tab content when no sessions"
        );
        assert!(
            !content.contains("▶"),
            "Should not show current tab indicator"
        );
    }

    #[test]
    fn test_session_manager_overlay_rendering() {
        // Create a test backend with a fixed size
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // Create test app with session manager active
        let mut app = create_test_app();
        app.focus_state = FocusState::SessionManager;

        // Add some test sessions
        app.tab_manager
            .add_session(crate::ssh::Host {
                name: "test-host-1".to_string(),
                destination: "user@host1.com".to_string(),
                user: None,
                port: None,
                aliases: String::new(),
                proxy_command: None,
            })
            .ok();

        app.tab_manager
            .add_session(crate::ssh::Host {
                name: "test-host-2".to_string(),
                destination: "user@host2.com".to_string(),
                user: None,
                port: None,
                aliases: String::new(),
                proxy_command: None,
            })
            .ok();

        // Set selection index
        app.session_manager_selection_index = 1;

        // Render the UI
        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();

        // Get the rendered content
        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<String>();

        // Check that session manager overlay is rendered
        assert!(
            content.contains("Session Manager"),
            "Should contain session manager title"
        );
        assert!(
            content.contains("test-host-1"),
            "Should contain first session name"
        );
        assert!(
            content.contains("test-host-2"),
            "Should contain second session name"
        );
        assert!(content.contains("Commands:"), "Should contain command help");
        assert!(
            content.contains("Enter=Switch"),
            "Should contain Enter command help"
        );
    }

    #[test]
    fn test_session_manager_empty_overlay() {
        // Create a test backend with a fixed size
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // Create test app with session manager active but no sessions
        let mut app = create_test_app();
        app.focus_state = FocusState::SessionManager;

        // Render the UI
        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();

        // Get the rendered content
        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<String>();

        // Check that session manager overlay shows empty message
        assert!(
            content.contains("Session Manager"),
            "Should contain session manager title"
        );
        assert!(
            content.contains("No active sessions"),
            "Should contain empty sessions message"
        );
    }
}
