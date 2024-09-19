use std::{any::Any, sync::RwLock};

use crop::Rope;
use spore_vm::{
    error::VmResult,
    val::{CustomType, NativeFunctionContext, ValBuilder},
    Vm,
};

#[derive(Debug, Default)]
pub struct SporeRope(RwLock<Rope>);

impl SporeRope {
    pub fn register(vm: Vm) -> Vm {
        vm.with_native_function("new-rope", new_rope)
            .with_native_function("rope->string", rope_to_string)
            .with_native_function("rope-clear!", rope_clear)
            .with_native_function("rope-append!", rope_append)
    }
}

fn new_rope(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    Ok(ctx.new_custom(SporeRope::default()))
}

fn rope_to_string(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let s = match ctx.arg(0).as_custom::<SporeRope>() {
        Some(r) => r.to_string(),
        None => todo!(),
    };
    Ok(ctx.new_string(s.into()))
}

fn rope_append(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let s = match ctx.arg(1).as_str() {
        Some(s) => s.to_string(),
        None => todo!(),
    };
    match ctx.arg(0).as_custom::<SporeRope>() {
        Some(r) => {
            let mut r = r.0.try_write().unwrap();
            let len = r.byte_len();
            r.insert(len, s);
        }
        None => todo!(),
    };

    let r = ctx.args()[0];
    // Unsafe OK: Internal value is from args which is guaranteed not to be garbage collected.
    Ok(unsafe { ctx.with_unsafe_val(r) })
}

fn rope_clear(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    match ctx.arg(0).as_custom::<SporeRope>() {
        Some(r) => {
            let mut r = r.0.try_write().unwrap();
            let len = r.byte_len();
            r.replace(0..len, "");
        }
        None => todo!(),
    };
    Ok(ctx.new_void())
}

impl CustomType for SporeRope {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Display for SporeRope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.try_read().unwrap().fmt(f)
    }
}
