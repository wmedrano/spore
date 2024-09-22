use std::{
    fmt::Write,
    fs::File,
    time::{Duration, Instant},
};

use buffer::SporeBuffer;
use compact_str::{format_compact, CompactString};
use crossterm::event::KeyEvent;
use log::*;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    DefaultTerminal,
};
use spore_vm::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, ValBuilder},
    DefaultDebugger, Settings, Vm,
};

mod buffer;

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
    let vm = Vm::new(Settings {
        enable_aggressive_inline: false,
    })
    .with_native_function("read-event!", read_event);
    let mut vm = SporeBuffer::register(vm);
    let main_src = r#"
(define *running?* (new-box true))
(define buffer (new-buffer "*scratch*" "Press <esc> to quit.\n"))

(define (running?) (unbox *running?*))
(define (quit!) (set-box! *running?* false))

(define (event-to-insert event)
  (if (= (string-length event) 1) (return event))
  (if (= event "<space>") (return " "))
  (if (= event "<enter>") (return "\n"))
  (return ""))

(define (handle-event-impl! event)
  (if (not event) (return void))
  (if (= event "<esc>") (return (quit!)))
  (if (= event "<backspace>") (return (buffer-delete! buffer)))
  (buffer-insert! buffer (event-to-insert event)))


(define (handle-event!)
  (handle-event-impl! (read-event!)))
"#;
    let start_t = Instant::now();
    vm.eval_str(main_src, &mut DefaultDebugger).unwrap();
    info!("Loading main in {:?}.", start_t.elapsed());
    vm
}

fn run(mut vm: Vm, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    let mut debugger = DefaultDebugger;
    let mut tmp_string = String::new();
    while vm
        .eval_function_by_name("running?", std::iter::empty(), &mut debugger)
        .unwrap()
        .is_truthy()
    {
        {
            let buffer = vm.val_by_name("buffer").unwrap();
            let buffer = buffer.as_custom::<SporeBuffer>(&vm).unwrap();
            terminal.draw(|frame| {
                let window_area = frame.area();
                let b = Block::default()
                    .title(buffer.name.as_str())
                    .border_style(Style::default().fg(Color::Magenta))
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL);
                let contents_area = b.inner(window_area);
                frame.render_widget(b, window_area);
                tmp_string.clear();
                tmp_string
                    .write_fmt(format_args!("{}", buffer.contents))
                    .unwrap();
                frame.render_widget(Paragraph::new(tmp_string.as_str()), contents_area);
            })?;
        }
        vm.eval_function_by_name("handle-event!", std::iter::empty(), &mut debugger)
            .unwrap();
    }
    info!("Exiting Spore.");
    Ok(())
}

/// The minimum frames per second. This occurs when no user events are detected.
const MINIMUM_FRAMES_PER_SECOND: u64 = 30;
/// The amount of time to wait for an event before moving on with rendering.
const READ_EVENT_TIMEOUT_DURATION: Duration =
    Duration::from_nanos(1_000_000_000 / MINIMUM_FRAMES_PER_SECOND);

fn read_event(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if !event::poll(READ_EVENT_TIMEOUT_DURATION).unwrap() {
        return Ok(ValBuilder::new(false.into()));
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
    Ok(ctx.new_string(event_str))
}
