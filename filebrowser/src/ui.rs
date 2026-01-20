use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;

pub fn draw(f: &mut Frame, app: &AppState) {
    let size = f.area();

    // Create main layout: title + content + footer
    let constraints = vec![
        Constraint::Length(1),    // Title
        Constraint::Min(0),       // Main content (browser or browser + stdout)
        Constraint::Length(1),    // Footer
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    // Title bar
    let title = Paragraph::new(format!("📁 {}", app.current_path.filename()))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    // Split content area if stdout is shown
    let (browser_area, stdout_area) = if app.show_stdout {
        let content_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);
        (content_split[0], Some(content_split[1]))
    } else {
        (chunks[1], None)
    };

    // Three-pane layout for browser
    let pane_constraints = if app.parent_entries.is_some() {
        // Three panes: parent (25%), current (50%), preview (25%)
        vec![
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ]
    } else {
        // Two panes at root: current (50%), preview (50%)
        vec![Constraint::Percentage(50), Constraint::Percentage(50)]
    };

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(pane_constraints)
        .split(browser_area);

    let mut pane_idx = 0;

    // Left pane: Parent directory (if not at root)
    if let Some(ref parent_entries) = app.parent_entries {
        let parent_items: Vec<ListItem> = parent_entries
            .iter()
            .map(|entry| {
                let prefix = if entry.is_dir { "📁 " } else { "📄 " };
                let content = format!("{}{}", prefix, entry.name);
                ListItem::new(content)
            })
            .collect();

        let parent_list = List::new(parent_items)
            .block(Block::default().borders(Borders::ALL).title("Parent"));
        f.render_widget(parent_list, panes[pane_idx]);
        pane_idx += 1;
    }

    // Middle pane: Current directory listing
    let items: Vec<ListItem> = app
        .current_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let prefix = if entry.is_dir { "📁 " } else { "📄 " };
            let content = format!("{}{}", prefix, entry.name);

            let style = if idx == app.cursor_position {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"));

    f.render_widget(list, panes[pane_idx]);
    pane_idx += 1;

    // Right pane: Preview
    let preview_text = match &app.preview_content {
        crate::app::PreviewContent::Directory(entries) => {
            entries
                .iter()
                .take(10)
                .map(|e| {
                    let prefix = if e.is_dir { "📁 " } else { "📄 " };
                    Line::from(format!("{}{}", prefix, e.name))
                })
                .collect::<Vec<_>>()
        }
        crate::app::PreviewContent::File(content) => {
            content
                .lines()
                .take(10)
                .map(|line| Line::from(line.to_string()))
                .collect::<Vec<_>>()
        }
        crate::app::PreviewContent::Loading => {
            vec![Line::from("Loading...")]
        }
        crate::app::PreviewContent::Error(e) => {
            vec![Line::from(format!("Error: {}", e))]
        }
    };

    let preview = Paragraph::new(preview_text)
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(preview, panes[pane_idx]);

    // Footer: Help text
    let help_text = "j/k: navigate | h: parent | l: enter | F4: toggle output | r: refresh | Ctrl+C: quit";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);

    // Display stdout if shown
    if let Some(stdout_area) = stdout_area {
        let stdout_lines: Vec<Line> = app.stdout_output
            .iter()
            .rev()
            .take(100)
            .rev()
            .map(|output_line| {
                // Parse [LEVEL] prefix and colorize it
                let text = &output_line.text;
                if let Some(bracket_end) = text.find(']') {
                    let level_part = &text[..bracket_end + 1]; // Includes the closing bracket
                    let message_part = &text[bracket_end + 1..].trim_start();

                    let level_color = match level_part {
                        "[TRACE]" => Color::DarkGray,
                        "[DEBUG]" => Color::Cyan,
                        "[INFO]" => Color::Blue,
                        "[WARN]" => Color::Yellow,
                        "[ERROR]" => Color::Red,
                        _ => Color::White,
                    };

                    let spans = vec![
                        Span::styled(level_part, Style::default().fg(level_color)),
                        Span::raw(format!(" {}", message_part)),
                    ];
                    Line::from(spans)
                } else {
                    Line::from(text.as_str())
                }
            })
            .collect();

        let stdout_widget = Paragraph::new(stdout_lines)
            .block(Block::default().borders(Borders::ALL).title("Output"))
            .scroll((0, 0));

        f.render_widget(stdout_widget, stdout_area);
    }

    // Display error if any
    if let Some(ref error) = app.error_message {
        let error_text = vec![Line::from(format!("❌ Error: {}", error))];
        let error_widget = Paragraph::new(error_text)
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(error_widget, chunks[1]);
    }
}
