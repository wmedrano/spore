use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    widgets::Paragraph,
    DefaultTerminal,
};
use spore_vm::{Vm, VmSettings};

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(terminal);
    ratatui::restore();
    app_result
}

fn run(mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    let mut vm = Vm::new(VmSettings {
        enable_aggressive_inline: false,
    });
    let greeting_src = r#"(define (greeting)
                              (list "Hello Spore!" "Press 'q' to quit." (working-directory)))"#;
    vm.eval_str(greeting_src)?;
    loop {
        let message = vm.eval_str("(greeting)")?.to_string();
        terminal.draw(move |frame| {
            frame.render_widget(Paragraph::new(message.as_str()), frame.area());
        })?;
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
