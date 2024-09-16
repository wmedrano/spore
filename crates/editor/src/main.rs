use std::io;

use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    widgets::Paragraph,
    DefaultTerminal,
};
use spore_vm::{Vm, VmSettings};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(terminal);
    ratatui::restore();
    app_result
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut vm = Vm::new(VmSettings {
        enable_aggressive_inline: false,
    });
    let wd = vm
        .eval_str("(working-directory)")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    loop {
        terminal.draw(|frame| {
            let greeting = Paragraph::new(format!(
                "Hello Ratatui! (press 'q' to quit)\nWorking Directory: {wd}"
            ));
            frame.render_widget(greeting, frame.area());
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
