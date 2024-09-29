use std::time::Duration;

use compact_str::{format_compact, CompactString};
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use log::*;
use spore_vm::{
    error::{VmError, VmResult},
    val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
    Vm,
};

pub fn register(vm: Vm) -> Vm {
    vm.with_native_function("read-event!", read_event)
        .with_native_function("special-event?", special_event_p)
}

/// The minimum frames per second. This occurs when no user events are detected.
const MINIMUM_FRAMES_PER_SECOND: u64 = 60;
/// The amount of time to wait for an event before moving on with rendering.
const READ_EVENT_TIMEOUT_DURATION: Duration =
    Duration::from_nanos(1_000_000_000 / MINIMUM_FRAMES_PER_SECOND);

fn read_event<'a>(ctx: NativeFunctionContext<'a>, _: &[Val]) -> VmResult<ValBuilder<'a>> {
    if !event::poll(READ_EVENT_TIMEOUT_DURATION).unwrap() {
        return Ok(ValBuilder::new(false.into()));
    }
    let event = event::read().map_err(|err| VmError::CustomError(err.to_string()))?;
    let mut is_special = true;
    let (mut event_key, modifiers): (CompactString, _) = match event {
        event::Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            modifiers,
            ..
        }) => {
            let code = match code {
                KeyCode::Backspace => "backspace".into(),
                KeyCode::Enter => "enter".into(),
                KeyCode::Left => "left".into(),
                KeyCode::Right => "right".into(),
                KeyCode::Up => "up".into(),
                KeyCode::Down => "down".into(),
                KeyCode::Home => "home".into(),
                KeyCode::End => "end".into(),
                KeyCode::PageUp => "page-up".into(),
                KeyCode::PageDown => "page-down".into(),
                KeyCode::Tab => "tab".into(),
                KeyCode::BackTab => "backtab".into(),
                KeyCode::Delete => "delete".into(),
                KeyCode::Insert => "insert".into(),
                KeyCode::F(n) => format_compact!("f{n}"),
                KeyCode::Char(' ') => "space".into(),
                KeyCode::Char(ch) => {
                    is_special = false;
                    format_compact!("{ch}")
                }
                KeyCode::Null => "null".into(),
                KeyCode::Esc => "esc".into(),
                KeyCode::CapsLock => "caps-lock".into(),
                KeyCode::ScrollLock => "scroll-lock".into(),
                KeyCode::NumLock => "num-lock".into(),
                KeyCode::PrintScreen => "print-screen".into(),
                KeyCode::Pause => "pause".into(),
                KeyCode::Menu => "menu".into(),
                KeyCode::KeypadBegin => "keypad-begin".into(),
                KeyCode::Media(_) => "unknown".into(),
                KeyCode::Modifier(_) => "unknown".into(),
            };
            (code, modifiers)
        }
        _ => ("unknown".into(), KeyModifiers::empty()),
    };
    if modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::ALT) {
        is_special = true;
    }
    if modifiers.contains(KeyModifiers::SHIFT) && is_special {
        event_key.insert_str(0, "shift-");
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        event_key.insert_str(0, "ctrl-");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        event_key.insert_str(0, "alt-");
    }
    if is_special {
        event_key.insert_str(0, "<");
        event_key.push_str(">");
    }
    if cfg!(debug_assertions) && event_key.is_heap_allocated() {
        warn!(
            "Generated large event string of length {len}. Large strings may negatively impact performance.",
            len=event_key.len());
    };
    Ok(ctx.new_string(event_key))
}

fn special_event_p<'a>(ctx: NativeFunctionContext<'a>, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    match args {
        [event] => {
            let event = event.try_str(ctx.vm()).map_err(|v| VmError::TypeError {
                src: None,
                context: "special-event?",
                expected: UnsafeVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
            let is_special = event.starts_with('<') && event.ends_with('>');
            Ok(ValBuilder::new(is_special.into()))
        }
        _ => Err(VmError::ArityError {
            function: "special-event?".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}
