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
    // If terminal is open, further split vertically
    let main_area = if app.is_terminal_open {
        let vert_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(chunks[1]);
        
        // Render terminal pane at the bottom
        render_terminal(f, app, vert_split[1]);
        vert_split[0]
    } else {
        chunks[1]
    };

    let main_ranks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_area);

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
            "Name: {}\nPath: {}\nSize: {} ({})\nModified: {}\n\n--- Metadata ---\nInode: {}\nPermissions: {}\nOwner: {}\nGroup: {}\n\n--- Description ---\n{}",
            entry.name,
            entry.path.display(),
            entry.human_size(),
            format!("{} bytes", entry.size),
            entry.mod_time.format("%Y-%m-%d %H:%M:%S"),
            entry.inode,
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
                " {} | {} | {} | {} | {} | {} ",
                app.config.get_hint("help"),
                app.config.get_hint("toggle_terminal"),
                app.config.get_hint("new_folder"),
                app.config.get_hint("page_up"),
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
        f.render_widget(ratatui::widgets::Clear, area);

        match prompt_type {
            crate::ui::app::PromptType::NewFolder => {
                let block = Paragraph::new(app.prompt_buffer.as_str())
                    .block(Block::default().borders(Borders::ALL).title(" New Folder Name ").border_style(Style::default().fg(Color::Yellow)));
                f.render_widget(block, area);
            }
            crate::ui::app::PromptType::DeleteConfirmation => {
                let block = Block::default().borders(Borders::ALL).title(" Delete Confirmation ").border_style(Style::default().fg(Color::Red));
                let inner = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // Question
                        Constraint::Min(0),    // Padding
                        Constraint::Length(3), // Buttons
                    ])
                    .split(inner);

                let question = Paragraph::new("Are you sure you want to delete?")
                    .alignment(ratatui::layout::Alignment::Center);

                let buttons_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(chunks[2]);

                let ok_style = if app.prompt_index == 0 {
                    Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                };
                let ok_btn = Paragraph::new("OK")
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).border_style(ok_style));

                let cancel_style = if app.prompt_index == 1 {
                    Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let cancel_btn = Paragraph::new("Cancel")
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).border_style(cancel_style));

                f.render_widget(block, area);
                f.render_widget(question, chunks[0]);
                f.render_widget(ok_btn, buttons_layout[0]);
                f.render_widget(cancel_btn, buttons_layout[1]);
            }
        }
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

fn render_terminal(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1)].as_ref())
        .split(area);

    let title = if app.terminal_focused {
        let cwd = app.manager.current_path().display().to_string();
        format!(" Terminal (focused) [{}] ", cwd)
    } else {
        format!(" Terminal [{} to focus] ", app.config.keybindings.terminal_prefix)
    };

    let border_style = if app.terminal_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);
        
    let inner_area = block.inner(chunks[0]);
    f.render_widget(block, chunks[0]);

    // Render VT100 screen only if PTY exists
    if let Some(parser) = &app.pty_parser {
        let screen = parser.screen();
        let hide_cursor = screen.hide_cursor();
        let (cy, cx) = screen.cursor_position();

        // Iterate over the visible area of the terminal
        for y in 0..inner_area.height {
            for x in 0..inner_area.width {
                if let Some(cell) = screen.cell(y, x) {
                     let ch = cell.contents();
                     if ch.trim().is_empty() && cell.bgcolor() == vt100::Color::Default {
                         continue;
                     }

                     let fg = map_color(cell.fgcolor());
                     let bg = map_color(cell.bgcolor());
                     
                     let mut style = Style::default().fg(fg).bg(bg);
                     if cell.bold() { style = style.add_modifier(Modifier::BOLD); }
                     if cell.italic() { style = style.add_modifier(Modifier::ITALIC); }
                     if cell.underline() { style = style.add_modifier(Modifier::UNDERLINED); }
                     if cell.inverse() { style = style.add_modifier(Modifier::REVERSED); }
                     
                     f.render_widget(Paragraph::new(ch).style(style), Rect::new(inner_area.x + x, inner_area.y + y, 1, 1));
                }
            }
        }

        // Render cursor
        if app.terminal_focused && !hide_cursor {
             // Check bounds
             if cy < inner_area.height && cx < inner_area.width {
                 f.set_cursor_position((inner_area.x + cx, inner_area.y + cy));
             }
        }
    } else {
        let text = " Terminal Closed (Ctrl+T to open) ";
        f.render_widget(Paragraph::new(text).alignment(ratatui::layout::Alignment::Center), inner_area);
    }
}

fn map_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
