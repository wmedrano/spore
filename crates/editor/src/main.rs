use std::{
    fs::File,
    io::{Read, Write},
    time::Instant,
};

use anyhow::Context;
use buffer::SporeBuffer;
use log::*;
use ratatui::{style::Style, DefaultTerminal};
use spore_vm::{Settings, Vm};
use widgets::WindowWidget;

mod buffer;
mod event;
mod widgets;

fn main() -> anyhow::Result<()> {
    init_logger();
    let vm = new_vm()?;
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(vm, terminal);
    ratatui::restore();
    app_result
}

fn new_vm() -> anyhow::Result<Vm> {
    let mut vm = Vm::new(Settings {
        enable_aggressive_inline: false,
    })
    .with(event::register)
    .with(SporeBuffer::register);
    let working_directory = std::env::current_dir().unwrap_or_default();
    let main_src_file = "main.spore";
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

fn run(mut vm: Vm, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    while vm
        .eval_function_by_name("running?", std::iter::empty())
        .unwrap()
        .is_truthy()
    {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.buffer_mut().set_style(area, Style::reset());
            let windows = vm.val_by_name("*windows*").unwrap();
            for window in windows.try_list(&vm).unwrap() {
                frame.render_widget(WindowWidget::new(&vm, *window), area);
            }
        })?;
        vm.eval_function_by_name("handle-event!", std::iter::empty())
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
