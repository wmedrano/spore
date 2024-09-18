use std::fs::File;

use crossterm::event::KeyEvent;
use log::info;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    widgets::Paragraph,
    DefaultTerminal,
};
use smol_str::{format_smolstr, SmolStr, ToSmolStr};
use spore_vm::{
    val::{NativeFunctionContext, ValBuilder},
    Vm, VmError, VmResult, VmSettings,
};

fn main() -> anyhow::Result<()> {
    init_logger();
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(terminal);
    ratatui::restore();
    app_result
}

fn init_logger() {
    let log_target =
        Box::new(File::create("/tmp/spore-editor.log").expect("Can't create log file"));
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(log_target))
        .init();
}

fn run(mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    let mut vm = Vm::new(VmSettings {
        enable_aggressive_inline: false,
    });
    vm.register_native_function("read-event", read_event);
    let main_src = r#"
(define (buffer-lines event)
    (list
        "Hello Spore!"
        (string-join (list "Working Directory: " (working-directory)))
        (string-join (list "Event: " event))
        "Press <esc> to quit."))
(define (handle-event event)
    (if (= event "<esc>")
        false
        (string-join (buffer-lines event) "\n")))
"#;
    vm.eval_str(main_src)?;
    let mut has_init = false;
    loop {
        let buffer_contents = if has_init {
            vm.eval_str("(handle-event (read-event))")?
        } else {
            has_init = true;
            vm.eval_str("(handle-event \"\")")?
        };
        let should_continue = buffer_contents.as_bool().unwrap_or(true);
        if !should_continue {
            info!("Exit requested.");
            return Ok(());
        }
        terminal.draw(move |frame| {
            frame.render_widget(
                Paragraph::new(buffer_contents.as_str().unwrap()),
                frame.area(),
            );
        })?;
    }
}

fn read_event(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let event = event::read().map_err(|err| VmError::CustomError(err.to_string()))?;
    let event_str: SmolStr = match event {
        event::Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            ..
        }) => match code {
            KeyCode::Backspace => "<backspace>".into(),
            KeyCode::Enter => "<enter>".into(),
            KeyCode::Left => "<left>".into(),
            KeyCode::Right => "<right>".into(),
            KeyCode::Up => "<up>".into(),
            KeyCode::Down => "<down>".into(),
            KeyCode::Home => "<home>".into(),
            KeyCode::End => "<end>".into(),
            KeyCode::PageUp => "<page-up>".into(),
            KeyCode::PageDown => "<page-down>".into(),
            KeyCode::Tab => "<tab>".into(),
            KeyCode::BackTab => "<backtab>".into(),
            KeyCode::Delete => "<delete>".into(),
            KeyCode::Insert => "<insert>".into(),
            KeyCode::F(n) => format_smolstr!("<f{n}>"),
            KeyCode::Char(' ') => "<space>".into(),
            KeyCode::Char(ch) => ch.to_smolstr(),
            KeyCode::Null => "<null>".into(),
            KeyCode::Esc => "<esc>".into(),
            KeyCode::CapsLock => "<caps-lock>".into(),
            KeyCode::ScrollLock => "<scroll-lock>".into(),
            KeyCode::NumLock => "<num-lock>".into(),
            KeyCode::PrintScreen => "<print-screen>".into(),
            KeyCode::Pause => "<pause>".into(),
            KeyCode::Menu => "<menu>".into(),
            KeyCode::KeypadBegin => "<keypad-begin".into(),
            KeyCode::Media(_) => "<unknown>".into(),
            KeyCode::Modifier(_) => "<unknown>".into(),
        },
        _ => return Ok(ValBuilder::new_void()),
    };
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_string(event_str) })
}
