use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Padding, Paragraph, Row, Table},
    text::{Line, Text},
    layout::Margin,
};
use style::palette::tailwind;

use super::app::{
    App, CURSOR_HORIZONTAL_PADDING, CURSOR_VERTICAL_OFFSET, FOOTER_HEIGHT,
    SEARCH_BAR_HEIGHT, SEARCHBAR_HORIZONTAL_PADDING, TABLE_HEADER_HEIGHT, INFO_TEXT,
    TABLE_MIN_HEIGHT,
};
use super::form::FormState;

/// Render the UI
pub fn ui(f: &mut Frame, app: &mut App) {
    match app.form_state {
        FormState::Hidden => render_main_ui(f, app),
        FormState::Active => render_form_ui(f, app),
    }
}

/// Render the main UI
fn render_main_ui(f: &mut Frame, app: &mut App) {
    let rects = Layout::vertical([
        Constraint::Length(SEARCH_BAR_HEIGHT),
        Constraint::Min(TABLE_MIN_HEIGHT),
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .split(f.area());

    render_searchbar(f, app, rects[0]);
    render_table(f, app, rects[1]);
    render_footer(f, app, rects[2]);

    // Show feedback message if present
    if let Some(message) = &app.feedback_message {
        render_feedback(f, message, app.is_feedback_error);
    }

    let mut cursor_position = rects[0].as_position();
    cursor_position.x += u16::try_from(app.search.cursor()).unwrap_or_default() + CURSOR_HORIZONTAL_PADDING;
    cursor_position.y += CURSOR_VERTICAL_OFFSET;

    f.set_cursor_position(cursor_position);
}

/// Render the form UI
#[allow(clippy::too_many_lines)]
fn render_form_ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    
    // Create a centered box for the form with additional space
    let form_width = 60;
    let form_height = 14; // Increased height
    let horizontal_margin = (area.width.saturating_sub(form_width)) / 2;
    let vertical_margin = (area.height.saturating_sub(form_height)) / 2;
    
    let form_area = Rect::new(
        horizontal_margin,
        vertical_margin,
        form_width,
        form_height,
    );
    
    // Create a block for the form
    let form_block = Block::default()
        .title("Add New SSH Host")
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
            .title("Port (optional)");
        
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
    
    // Render help text
    let help_text = "(Tab) next field | (Shift+Tab) previous field | (Enter) save | (Esc) cancel";
    let help_paragraph = Paragraph::new(Line::from(help_text))
        .alignment(Alignment::Center)
        .style(Style::new().fg(app.palette.c300));
    
    let help_area = Rect::new(
        horizontal_margin,
        vertical_margin + form_height,
        form_width,
        1,
    );
    
    f.render_widget(help_paragraph, help_area);
    
    // Show feedback message if present
    if let Some(message) = &app.feedback_message {
        render_feedback(f, message, app.is_feedback_error);
    }
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
    let info_footer = Paragraph::new(Line::from(app.search.value())).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.palette.c400))
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

/// Render the footer
pub fn render_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let info_footer = Paragraph::new(Line::from(INFO_TEXT)).centered().block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.palette.c400))
            .border_type(BorderType::Rounded),
    );
    f.render_widget(info_footer, area);
}