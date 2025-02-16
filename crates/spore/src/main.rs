use clap::{Parser, ValueEnum};
use ratatui::{DefaultTerminal, Frame};
use spore_vm::{val::Val, vm::Vm};

mod buffer;
mod events;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_enum, short, long, default_value = "editor")]
    pub mode: Mode,
}

#[derive(Copy, Clone, Parser, PartialEq, Default, Debug, ValueEnum)]
pub enum Mode {
    #[default]
    Editor,
    Repl,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut vm = Vm::default().with(buffer::register_buffer);
    match args.mode {
        Mode::Editor => {
            let terminal = ratatui::init();
            let result = run(&mut vm, terminal);
            ratatui::restore();
            result
        }
        Mode::Repl => {
            spore_repl::Repl::new(vm).run()?;
            Ok(())
        }
    }
}

fn run(vm: &mut Vm, mut terminal: DefaultTerminal) -> Result<(), Box<dyn std::error::Error>> {
    vm.eval_str(
        r#"
(define exit? false)
(define text (new-buffer ""))
(define cursor 0)

(define (handle-event! event)
  (if (= event "<esc>")
    (define exit? true)
    (do
      (buffer-insert! text cursor event)
      (define cursor (+ cursor (string-len event))))))
"#,
    )
    .unwrap();
    while !vm
        .get_global_by_name("exit?")
        .unwrap_or_default()
        .is_truthy()
    {
        let text_val = vm.eval_str("(buffer->string text)").unwrap();
        let text = text_val.as_str(&vm).unwrap();
        terminal.draw(|frame: &mut Frame| frame.render_widget(text, frame.area()))?;
        handle_events(vm);
    }
    Ok(())
}

fn handle_events(vm: &mut Vm) {
    vm.set_global_by_name("tmp-event", Val::Void);
    for event in events::events() {
        let s = vm.make_string(event.clone());
        vm.set_global_by_name("tmp-event", s);
        vm.eval_str("(handle-event! tmp-event)").unwrap();
    }
}
