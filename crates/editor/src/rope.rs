use std::{any::Any, sync::RwLock};

use crop::Rope;
use smol_str::ToSmolStr;
use spore_vm::{
    val::{CustomType, NativeFunctionContext, ValBuilder},
    Vm, VmResult,
};

#[derive(Debug, Default)]
pub struct SporeRope(RwLock<Rope>);

impl SporeRope {
    pub fn register(vm: &mut Vm) {
        vm.register_native_function("new-rope", new_rope);
        vm.register_native_function("rope->string", rope_to_string);
        vm.register_native_function("rope-clear!", rope_clear);
        vm.register_native_function("rope-append!", rope_append);
    }
}

fn new_rope(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_custom(SporeRope::default()) })
}

fn rope_to_string(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let s = match ctx.arg(0).as_custom::<SporeRope>() {
        Some(r) => r.to_string(),
        None => todo!(),
    };
    // Unsafe OK: Value is returned immediately.
    Ok(unsafe { ctx.new_string(s.to_smolstr()) })
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
    // Unsafe OK: Internal value is from args which is guaranteed not to be garbage collected.
    Ok(unsafe { ValBuilder::new_internal(ctx.args()[0]) })
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
    // Unsafe OK: Value is returned immediately.
    Ok(ValBuilder::new_void())
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
