use log::error;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Widget},
};
use spore_vm::{
    val::{UnsafeVal, Val},
    Vm,
};

use crate::buffer::SporeBuffer;

pub struct WindowWidget<'a> {
    vm: &'a Vm,
    val: Val<'a>,
}

impl<'a> WindowWidget<'a> {
    pub fn new(vm: &'a Vm, window: Val<'a>) -> WindowWidget<'a> {
        WindowWidget { vm, val: window }
    }
}

impl<'a> Widget for WindowWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.val.is_struct() {
            error!(
                "Expected window to be of type struct but got {struct_type} with value {value}",
                struct_type = self.val.type_name(),
                value = self.val.format_quoted(self.vm)
            );
            return;
        }
        let get_field = |name: &str| -> Option<u16> {
            let field_val = match self.val.try_struct_get(self.vm, name).unwrap() {
                Some(v) => v,
                None => {
                    error!("window struct is missing field {name:?}.");
                    return None;
                }
            };
            match field_val.try_int() {
                Ok(i) if (0..=u16::MAX as i64).contains(&i) => Some(i as u16),
                Ok(i) => {
                    error!("window struct field {name:?} is out of range with value {i}.");
                    None
                }
                Err(v) => {
                    error!("window struct expected field {name:?} of type {expected_type}, but got type {got_type}: {val}",
                           expected_type = UnsafeVal::INT_TYPE_NAME,
                           got_type = v.type_name(),
                           val = v.format_quoted(self.vm));
                    None
                }
            }
        };
        let mut render_impl = || -> Option<()> {
            let area = area.intersection(Rect::new(
                get_field("x")?,
                get_field("y")?,
                get_field("width")?,
                get_field("height")?,
            ));
            let buffer_val = match self.val.try_struct_get(self.vm, "buffer").unwrap() {
                Some(b) => b,
                None => {
                    error!("window struct did not have field {:?}.", "buffer");
                    return None;
                }
            };
            let buffer = buffer_val
                .try_custom::<SporeBuffer>(self.vm)
                .inspect_err(|err| {
                    error!("window struct failed to get buffer: {err}");
                })
                .ok()?;
            let draw_cursor = self
                .val
                .try_struct_get(self.vm, "draw-cursor?")
                .unwrap()
                .map(|v| v.is_truthy())
                .unwrap_or(false);
            BufferWidget::new(&buffer, draw_cursor).render(area, buf);
            Some(())
        };
        let _ = render_impl();
    }
}

pub struct BufferWidget<'a> {
    buffer: &'a SporeBuffer,
    draw_cursor: bool,
}

impl<'a> BufferWidget<'a> {
    pub fn new(buffer: &'a SporeBuffer, draw_cursor: bool) -> Self {
        BufferWidget {
            buffer,
            draw_cursor,
        }
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
        Clear.render(area, buf);
        let b = Block::default()
            .title(self.buffer.name.as_str())
            .border_style(Style::default().fg(Color::White))
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
            if self.draw_cursor && byte_range.contains(&(self.buffer.cursor.byte_idx())) {
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
