use std::{fs::File, time::Duration};

use compact_str::{format_compact, CompactString};
use crossterm::event::KeyEvent;
use log::{error, warn};
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    widgets::Paragraph,
    DefaultTerminal,
};
use rope::SporeRope;
use spore_vm::{
    val::{NativeFunctionContext, ValBuilder},
    Vm, VmError, VmResult, VmSettings,
};

mod rope;

fn main() -> anyhow::Result<()> {
    init_logger();
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(new_vm(), terminal);
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

fn new_vm() -> Vm {
    let vm = Vm::new(VmSettings {
        enable_aggressive_inline: false,
    })
    .with_native_function("read-event", read_event);
    SporeRope::register(vm)
}

fn run(mut vm: Vm, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    let main_src = r#"
(define buffer-contents (new-rope))
(define current-event (new-box ""))

(define (reset-buffer! event)
    (rope-clear!  buffer-contents buffer-contents)
    (rope-append! buffer-contents "Hello Spore!\n")
    (rope-append! buffer-contents "Working Directory: ")
    (rope-append! buffer-contents (working-directory))
    (rope-append! buffer-contents "\nEvent: ")
    (rope-append! buffer-contents event)
    (rope-append! buffer-contents "\nPress <esc> to quit."))

(define (handle-event event)
    (if (= event "<esc>") (return false))
    (if (truthy? event) (set-box! current-event event))
    (reset-buffer! (unbox current-event))
    (rope->string buffer-contents))
"#;
    vm.eval_str(main_src)?;
    loop {
        let buffer_contents = vm
            .eval_str("(handle-event (read-event))")
            .inspect_err(|err| error!("VM Failed: {err}"))?;
        let should_continue = buffer_contents.as_bool().unwrap_or(true);
        if !should_continue {
            break;
        }
        terminal.draw(move |frame| {
            frame.render_widget(
                Paragraph::new(buffer_contents.as_str().unwrap()),
                frame.area(),
            );
        })?;
    }
    Ok(())
}

fn read_event(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if !event::poll(Duration::from_millis(10)).unwrap() {
        return Ok(ValBuilder::new_bool(false));
    };
    let event = event::read().map_err(|err| VmError::CustomError(err.to_string()))?;
    let event_str: CompactString = match event {
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
            KeyCode::F(n) => format_compact!("<f{n}>"),
            KeyCode::Char(' ') => "<space>".into(),
            KeyCode::Char(ch) => format_compact!("{ch}"),
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
        _ => "".into(),
    };
    if cfg!(debug_assertions) && event_str.is_heap_allocated() {
        warn!("Generated large event string of length {len}. Large strings may negatively impact performance.", len=event_str.len());
    };
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_string(event_str) })
}
