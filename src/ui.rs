use crate::app::App;
use minigem::{Line, LineKind};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_page<B: Backend>(f: &mut Frame<B>, app: &App, layout_chunk: Rect) {
    let text: Vec<Spans> = app
        .page
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| span_from_line(line, app, index))
        .collect();
    let block = Block::default()
        .title(&app.address[..])
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(text).block(block).scroll((app.scroll, 0));
    f.render_widget(paragraph, layout_chunk)
}

fn span_from_line<'a>(line: &'a Line, app: &'a App, index: usize) -> Spans<'a> {
    match line.kind() {
        LineKind::Text => Spans::from(line.text().unwrap()),
        LineKind::Heading => {
            let s = format!(
                "{} {}",
                "#".repeat(line.level().unwrap()),
                line.text().unwrap()
            );
            Spans::from(s)
        }
        LineKind::UnorderedListItem => {
            let s = format!("  - {}", line.text().unwrap());
            Spans::from(s)
        }
        LineKind::Link => {
            let content = format!(
                "=> [{}] {} {}",
                app.page.link_numbers.get(&index).unwrap(),
                line.link().unwrap(),
                line.text().unwrap_or("")
            );
            match app.page.highlighted_link {
                Some(i) if index == app.page.link_indices[i] => {
                    Spans::from(Span::styled(content, Style::default().bg(Color::Cyan)))
                }
                _ => Spans::from(content),
            }
        }
        _ => Spans::from("unsupported line type"),
    }
}
