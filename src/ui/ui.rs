use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, ListState},
    Frame,
};
use crate::ui::app::{App, InputMode};

pub fn render(f: &mut Frame, app: &mut App) {
    match &app.input_mode {
        InputMode::Config | InputMode::Remapping(_) => render_config(f, app),
        _ => render_main(f, app),
    }
}

fn render_main(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main area
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    // Header (Path + Search)
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(45), // Wider search bar
        ])
        .split(chunks[0]);

    let header = Paragraph::new(format!(" Xplore - {}", app.manager.current_path().display()))
        .block(Block::default().borders(Borders::ALL).title("Path"));
    f.render_widget(header, header_chunks[0]);

    let search_title = if app.is_searching {
        " Searching... ".to_string()
    } else if let InputMode::Search = app.input_mode {
        " Global Search (Enter to scan /) ".to_string()
    } else {
        format!(" Global Search {} ", app.config.keybindings.search)
    };
    let search_border_style = if let InputMode::Search = app.input_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let search_bar = Paragraph::new(format!(" {}", app.search_query))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(search_title)
            .border_style(search_border_style));
    f.render_widget(search_bar, header_chunks[1]);

    // Main area (Split horizontally: List | Details)
    let main_ranks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60), // More room for details/description
        ])
        .split(chunks[1]);

    // File List
    let items: Vec<ListItem> = app.filtered_entries.iter().map(|e| {
        let prefix = if e.is_dir { "[DIR] " } else { "      " };
        let style = if e.is_dir { 
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) 
        } else { 
            Style::default() 
        };
        ListItem::new(format!("{}{}", prefix, e.name)).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(list, main_ranks[0], &mut state);

    // Details Panel
    if let Some(entry) = app.filtered_entries.get(app.selected_index) {
        let desc = entry.description.as_deref().unwrap_or("No description");
        let details_text = format!(
            "Name: {}\nPath: {}\nSize: {} bytes\nModified: {}\n\n--- Description ---\n{}",
            entry.name,
            entry.path.display(),
            entry.size, 
            entry.mod_time.format("%Y-%m-%d %H:%M:%S"),
            desc
        );
        let details = Paragraph::new(details_text)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(details, main_ranks[1]);
    }

    // Edit Mask (Popup)
    if let InputMode::Editing = app.input_mode {
        let area = centered_rect(80, 60, f.area());
        let edit_block = Paragraph::new(app.edit_buffer.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Edit Description (F2: Save, Esc: Cancel) ")
                .border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(ratatui::widgets::Clear, area); // Clear the background
        f.render_widget(edit_block, area);
    }

    // Footer
    let footer_text = match app.input_mode {
        InputMode::Editing => " [Enter] Newline | [F2] Save | [Esc] Cancel ".to_string(),
        InputMode::Search => " [Chars] Query | [Enter] DEEP GLOBAL SEARCH | [Esc] Cancel ".to_string(),
        _ => {
            format!(
                " {} | {} | {} | {} | {} ",
                app.config.get_hint("quit"),
                app.config.get_hint("search"),
                app.config.get_hint("edit"),
                app.config.get_hint("enter"),
                app.config.get_hint("settings")
            )
        }
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

fn render_config(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // List
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    let header = Paragraph::new(" Keybinding Settings ")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let actions = app.config.get_actions();
    let items: Vec<ListItem> = actions.iter()
        .map(|(action, key)| {
            ListItem::new(format!("{:<15} : {}", action, key))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Select an action to remap"))
        .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(Some(app.config_index));
    f.render_stateful_widget(list, chunks[1], &mut state);

    if let InputMode::Remapping(action) = &app.input_mode {
        let area = centered_rect(50, 30, f.area());
        
        let text = if let Some(err) = &app.error_message {
            format!(" ERROR: {}\n\n Press ANY KEY for [{}] ", err, action)
        } else {
            format!(" Press NEW KEY for [{}] \n\n (Press Esc to cancel) ", action)
        };

        let block = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)))
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(block, area);
    }

    let footer = Paragraph::new(" [Enter] Remap | [Esc] Back to Files ")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
