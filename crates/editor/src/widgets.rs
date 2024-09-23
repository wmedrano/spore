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
        buf.set_style(area, Style::new().fg(Color::White).bg(Color::Black));
        let (mut x, mut width) = (area.x, area.width);
        let (mut y, mut height) = (area.y, area.height);
        let mut start_byte = 0;
        for grapheme in self
            .buffer
            .contents
            .graphemes()
            .chain(std::iter::once(" ".into()))
        {
            let end_byte = start_byte + grapheme.as_ref().len();
            if (start_byte..end_byte).contains(&(self.buffer.cursor_offset(0) as usize)) {
                buf[(x, y)].set_bg(Color::Magenta);
            }
            start_byte = end_byte;
            match grapheme.as_ref() {
                "\n" => {
                    (x, width) = (area.x, area.width);
                    (y, height) = (y + 1, height - 1);
                    if height == 0 {
                        return;
                    }
                }
                sym => {
                    buf[(x, y)].set_symbol(sym);
                    if width <= 1 {
                        (x, width) = (area.x, area.width);
                        (y, height) = (y + 1, height - 1);
                        if height == 0 {
                            return;
                        }
                    } else {
                        (x, width) = (x + 1, width - 1);
                    }
                }
            }
        }
    }
}
