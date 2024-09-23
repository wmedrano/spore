use std::{
    fs::File,
    io::{Read, Write},
    time::Instant,
};

use buffer::SporeBuffer;
use log::*;
use ratatui::DefaultTerminal;
use spore_vm::{DefaultDebugger, Settings, Vm};
use widgets::BufferWidget;

mod buffer;
mod event;
mod widgets;

fn main() -> anyhow::Result<()> {
    init_logger();
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(new_vm(), terminal);
    ratatui::restore();
    app_result
}

fn new_vm() -> Vm {
    let mut vm = Vm::new(Settings {
        enable_aggressive_inline: false,
    })
    .with(event::register)
    .with(SporeBuffer::register);
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
  (if (= event "<left>") (return (buffer-cursor-move! buffer -1)))
  (if (= event "<right>") (return (buffer-cursor-move! buffer 1)))
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
    while vm
        .eval_function_by_name("running?", std::iter::empty(), &mut debugger)
        .unwrap()
        .is_truthy()
    {
        {
            let buffer = vm.val_by_name("buffer").unwrap();
            let buffer = buffer.as_custom::<SporeBuffer>(&vm).unwrap();
            terminal.draw(|frame| {
                frame.render_widget(BufferWidget::new(&buffer), frame.area());
            })?;
        }
        vm.eval_function_by_name("handle-event!", std::iter::empty(), &mut debugger)
            .unwrap();
    }
    info!("Exiting Spore.");
    Ok(())
}

pub const LOG_FILE: &str = "/tmp/spore-editor.log";

fn print_logs() -> anyhow::Result<()> {
    let file = File::open(LOG_FILE)?;
    let mut reader = std::io::BufReader::new(file);

    // Read and print the file contents in chunks
    let mut buffer = [0; 1024]; // 1KB buffer
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        std::io::stderr().write_all(&buffer[..bytes_read])?;
    }
    Ok(())
}

fn init_logger() {
    let log_target = Box::new(File::create(LOG_FILE).expect("Can't create log file"));
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(log_target))
        .init();
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        print_logs().ok();
        previous_hook(info);
    }));
}
