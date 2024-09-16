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
    vm.eval_str(
        "(define (greeting) (list \"Hello Spore!\" \"Press 'q' to quit.\" (working-directory)))",
    )
    .unwrap();
    loop {
        let message = vm.eval_str("(greeting)").unwrap().to_string();
        terminal.draw(move |frame| {
            let greeting = Paragraph::new(message.as_str());
            frame.render_widget(greeting, frame.area());
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
