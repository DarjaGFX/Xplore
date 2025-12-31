use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::ui::app::{App, InputMode};

pub fn render(f: &mut Frame, app: &mut App) {
    match &app.input_mode {
        InputMode::Config | InputMode::Remapping(_) => render_config(f, app),
        InputMode::Prompt(_) => {
            render_main(f, app);
            render_prompt(f, app);
        }
        InputMode::Help => {
            render_main(f, app);
            render_help(f, app);
        }
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
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    // File List
    let items: Vec<ListItem> = app.filtered_entries.iter().map(|e| {
        let prefix = if e.is_dir { "[DIR] " } else { "      " };
        let mut style = if e.is_dir { 
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) 
        } else { 
            Style::default() 
        };
        
        if app.is_selected(&e.path) {
            style = style.bg(Color::Rgb(50, 50, 50)).add_modifier(Modifier::ITALIC);
        }

        let name = if app.is_selected(&e.path) {
            format!("* {}", e.name)
        } else {
            e.name.clone()
        };

        ListItem::new(format!("{}{}", prefix, name)).style(style)
    }).collect();

    // Track list height for Home/End/Page calculation
    app.list_height = main_ranks[0].height.saturating_sub(2); // Subtract borders

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, main_ranks[0], &mut app.list_state);

    // Details Panel
    if let Some(entry) = app.filtered_entries.get(app.selected_index) {
        let desc = entry.description.as_deref().unwrap_or("No description");
        let details_text = format!(
            "Name: {}\nPath: {}\nSize: {} ({})\nModified: {}\n\n--- Metadata ---\nPermissions: {}\nOwner: {}\nGroup: {}\n\n--- Description ---\n{}",
            entry.name,
            entry.path.display(),
            entry.human_size(),
            format!("{} bytes", entry.size),
            entry.mod_time.format("%Y-%m-%d %H:%M:%S"),
            entry.permissions,
            entry.owner,
            entry.group,
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
        InputMode::Prompt(_) => " [Chars] Input | [Enter] OK | [Esc] Cancel ".to_string(),
        _ => {
            format!(
                " {} | {} | {} | {} | {} | {} | {} ",
                app.config.get_hint("help"),
                app.config.get_hint("select"),
                app.config.get_hint("new_folder"),
                app.config.get_hint("page_up"),
                app.config.get_hint("page_down"),
                app.config.get_hint("search"),
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

    let categorized = app.config.get_categorized_actions();
    let mut items = Vec::new();
    let mut flat_index = 0;
    
    for (category, actions) in categorized {
        items.push(ListItem::new(format!("--- {} ---", category)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        for (action, key) in actions {
            let style = if flat_index == app.config_index {
                Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            items.push(ListItem::new(format!("  {:<15} : {}", action, key)).style(style));
            flat_index += 1;
        }
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Select an action to remap"));

    f.render_widget(list, chunks[1]);

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

fn render_prompt(f: &mut Frame, app: &mut App) {
    if let InputMode::Prompt(prompt_type) = &app.input_mode {
        let area = centered_rect(60, 20, f.area());
        let title = match prompt_type {
            crate::ui::app::PromptType::NewFolder => " New Folder Name ",
            crate::ui::app::PromptType::DeleteConfirmation => " Delete Selected? (y/n) ",
        };
        let block = Paragraph::new(app.prompt_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title(title).border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(block, area);
    }
}

fn render_help(f: &mut Frame, app: &mut App) {
    let area = centered_rect(80, 80, f.area());
    let mut help_text = String::from(" --- Xplore Help ---\n\n");
    
    let categorized = app.config.get_categorized_actions();
    for (category, actions) in categorized {
        help_text.push_str(&format!("  [{}]\n", category));
        for (action, key) in actions {
            help_text.push_str(&format!("    {:<15} : {}\n", action, key));
        }
        help_text.push('\n');
    }
    help_text.push_str("\n Press Esc or F1 to close ");

    let block = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(block, area);
}
