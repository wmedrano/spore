use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Widget},
};

use crate::buffer::SporeBuffer;

pub struct BufferWidget<'a> {
    buffer: &'a SporeBuffer,
}

impl<'a> BufferWidget<'a> {
    pub fn new(buffer: &'a SporeBuffer) -> Self {
        BufferWidget { buffer }
    }
}

impl<'a> Widget for BufferWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let contents_area = self.render_border(area, buf);
        self.render_text(contents_area, buf);
    }
}

impl<'a> BufferWidget<'a> {
    fn render_border(&self, area: Rect, buf: &mut Buffer) -> Rect {
        let b = Block::default()
            .title(self.buffer.name.as_str())
            .border_style(Style::default().fg(Color::LightCyan))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        let inner_area = b.inner(area);
        b.render(area, buf);
        inner_area
    }

    fn render_text(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let (mut x, mut width) = (area.x, area.width);
        let (mut y, mut height) = (area.y, area.height);
        let mut byte_range = 0..0;
        let graphemes = self
            .buffer
            .contents
            .graphemes()
            // Required for cursor at the end of the text.
            .chain(std::iter::once(" ".into()));
        for grapheme in graphemes {
            let cell = &mut buf[(x, y)];
            byte_range = byte_range.end..byte_range.end + grapheme.as_ref().len();
            if byte_range.contains(&(self.buffer.cursor.byte_idx())) {
                cell.set_bg(Color::Magenta);
            } else {
                cell.set_fg(Color::White);
                cell.set_bg(Color::Black);
            }
            match grapheme.as_ref() {
                "\n" => {
                    (x, width) = (area.x, area.width);
                    (y, height) = (y + 1, height - 1);
                    if height == 0 {
                        return;
                    }
                }
                sym => {
                    let cell_width = match sym {
                        "\t" => {
                            cell.set_symbol(" ");
                            4
                        }
                        _ => {
                            cell.set_symbol(sym);
                            1
                        }
                    };
                    if width <= cell_width {
                        (x, width) = (area.x, area.width);
                        (y, height) = (y + 1, height - 1);
                        if height == 0 {
                            return;
                        }
                    } else {
                        (x, width) = (x + cell_width, width - cell_width);
                    }
                }
            }
        }
    }
}
