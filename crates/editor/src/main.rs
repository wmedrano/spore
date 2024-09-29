use std::{
    fs::File,
    io::{Read, Write},
    time::Instant,
};

use anyhow::Context;
use buffer::SporeBuffer;
use log::*;
use ratatui::{style::Style, Terminal};
use spore_vm::{val::Val, Settings, Vm};
use widgets::WindowWidget;

mod buffer;
mod event;
mod widgets;

fn main() -> anyhow::Result<()> {
    init_logger();
    let vm = new_vm("main.spore")?;
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(vm, terminal);
    ratatui::restore();
    app_result
}

fn new_vm(main_src_file: &str) -> anyhow::Result<Vm> {
    let mut vm = Vm::new(Settings {
        enable_aggressive_inline: false,
        enable_source_maps: true,
    })
    .with(event::register)
    .with(SporeBuffer::register);
    let working_directory = std::env::current_dir().unwrap_or_default();
    let start_t = Instant::now();
    let main_src = std::fs::read_to_string(main_src_file)
        .with_context(|| format!("Not found under working directory: {working_directory:?}"))
        .with_context(|| format!("Failed to open {main_src_file:?}"))?;
    info!(
        "Loaded source code from {main_src_file:?} with working directory {working_directory:?} in {duration:?}.",
        duration = start_t.elapsed()
    );
    let start_t = Instant::now();
    vm.eval_str(&main_src).unwrap();
    info!("Evaluated main in {:?}.", start_t.elapsed());
    Ok(vm)
}

fn run<T: ratatui::backend::Backend>(mut vm: Vm, mut terminal: Terminal<T>) -> anyhow::Result<()> {
    let mut terminal_size = terminal.size()?;
    let mut terminal_size_changed = false;
    vm.eval_function_by_name(
        "handle-size-change!",
        [
            Val::new_int(terminal_size.width.into()),
            Val::new_int(terminal_size.height.into()),
        ]
        .into_iter(),
    )?;
    while vm
        .eval_function_by_name("running?", std::iter::empty())
        .unwrap()
        .is_truthy()
    {
        let mut windows = vm.eval_function_by_name("windows", std::iter::empty())?;
        let (vm, windows) = windows.split();
        terminal.draw(|frame| {
            let area = frame.area();
            if terminal_size != area.as_size() {
                let function_args = [
                    Val::new_int(area.width as _),
                    Val::new_int(area.height as _),
                ];
                let res =
                    vm.eval_function_by_name("handle-size-change!", function_args.into_iter());
                if let Err(err) = res {
                    error!("Terminal resize with handle-size-changed! failed: {err}")
                }
            }
            frame.buffer_mut().set_style(area, Style::reset());
            for window in windows.try_list(vm).unwrap() {
                frame.render_widget(WindowWidget::new(vm, *window), area);
            }
            let frame_size = area.as_size();
            if frame_size != terminal_size {
                terminal_size = frame_size;
                terminal_size_changed = true;
            }
        })?;
        vm.eval_function_by_name("handle-event!", std::iter::empty())?;
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

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn ctrl_q_quits() {
        let mut vm = new_vm("../../main.spore").unwrap();
        let read_event_override = r#"(define (read-event!) "<ctrl-q>")"#;
        vm.eval_str(read_event_override).unwrap();
        let terminal = ratatui::Terminal::new(TestBackend::new(80, 40)).unwrap();
        run(vm, terminal).unwrap();
    }

    #[test]
    fn terminal_size_0_is_ok() {
        let mut vm = new_vm("../../main.spore").unwrap();
        let read_event_override = r#"(define (read-event!) "<ctrl-q>")"#;
        vm.eval_str(read_event_override).unwrap();
        let terminal = ratatui::Terminal::new(TestBackend::new(0, 0)).unwrap();
        run(vm, terminal).unwrap();
    }
}
