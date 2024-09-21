use std::{fs::File, time::Duration};

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
    Vm, VmSettings,
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
    let vm = Vm::new(VmSettings {
        enable_aggressive_inline: false,
    })
    .with_native_function("read-event!", read_event);
    SporeBuffer::register(vm)
}

fn run(mut vm: Vm, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    let main_src = r#"
(define running? (new-box true))
(define buffer (new-buffer "*scratch*" "Press <esc> to quit.\n"))

(define (handle-event-impl! event)
  (if (not event) (return void))
  (if (= event "<esc>") (set-box! running? false))
  (if (= event "<esc>") (return void))
  (buffer-append! buffer "Pressed ")
  (buffer-append! buffer event)
  (buffer-append! buffer ".\n"))


(define (handle-event!)
  (handle-event-impl! (read-event!)))
"#;
    vm.eval_str(main_src).unwrap();
    while vm.eval_str("(unbox running?)").unwrap().as_bool().unwrap() {
        {
            let buffer = vm.eval_str("buffer").unwrap();
            let buffer = buffer
                .as_custom::<SporeBuffer>()
                .unwrap()
                .0
                .try_read()
                .unwrap();
            terminal.draw(|frame| {
                let window_area = frame.area();
                let b = Block::default()
                    .title(buffer.name.as_str())
                    .border_style(Style::default().fg(Color::Magenta))
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL);
                let contents_area = b.inner(window_area);
                frame.render_widget(b, window_area);
                frame.render_widget(Paragraph::new(buffer.contents.to_string()), contents_area);
            })?;
        }
        vm.eval_str("(handle-event!)").unwrap();
    }
    info!("Exiting Spore.");
    Ok(())
}

fn read_event(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if !event::poll(Duration::from_millis(10)).unwrap() {
        return Ok(ctx.new_bool(false));
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
