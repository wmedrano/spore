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
    let main_src = r#"
(define (buffer-lines)
    (list
        "Hello Spore!"
        (string-join (list "Working Directory: " (working-directory)))
        "Press 'q' to quit."))
(define (buffer-contents)
    (string-join (buffer-lines) "\n"))
"#;
    vm.eval_str(main_src)?;
    loop {
        let buffer_contents = vm.eval_str("(buffer-contents)")?;
        terminal.draw(move |frame| {
            frame.render_widget(
                Paragraph::new(buffer_contents.as_str().unwrap()),
                frame.area(),
            );
        })?;
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
