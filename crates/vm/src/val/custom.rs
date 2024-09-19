use std::any::Any;

/// Contains a custom value.
#[derive(Debug)]
pub struct CustomVal(Box<dyn CustomType>);

impl CustomVal {
    /// Create a new `CustomVal` from any type that implements `CustomType`.
    pub fn new(val: impl CustomType) -> CustomVal {
        CustomVal(Box::new(val))
    }

    /// Get the underlying value of [Self] if it is of type `T` or else return `None`.
    pub fn get<T>(&self) -> Option<&T>
    where
        T: CustomType,
    {
        self.0.as_ref().as_any().type_id();
        self.0.as_ref().as_any().downcast_ref::<T>()
    }
}

impl std::fmt::Display for CustomVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{inner}", inner = self.0)
    }
}

/// A trait that defines a value that can be created or referenced within the VM.
/// ```rust
/// #[derive(Debug, Default)]
/// pub struct MyType(i64);
/// impl spore_vm::val::CustomType for MyType {
///     fn as_any(&self) -> &dyn std::any::Any {
///         self
///     }
/// }
/// impl std::fmt::Display for MyType {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "my number is {}", self.0)
///     }
/// }
/// ```
pub trait CustomType: 'static + std::fmt::Display + std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        val::{NativeFunctionContext, ValBuilder},
        Vm, VmResult,
    };

    #[derive(Debug, PartialEq)]
    struct MyType {
        number: i64,
    }

    impl CustomType for MyType {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl std::fmt::Display for MyType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "magic number {}", self.number)
        }
    }

    #[test]
    fn custom_type_can_be_printed() {
        let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
        assert_eq!(
            vm.eval_str("custom-value").unwrap().to_string(),
            "magic number 42"
        );
    }

    #[test]
    fn custom_type_can_be_accessed() {
        let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
        assert_eq!(
            vm.eval_str("custom-value").unwrap().as_custom::<MyType>(),
            Some(&MyType { number: 42 })
        );
    }

    #[test]
    fn custom_type_can_be_made_from_native_function() {
        fn custom_function(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
            let number = ctx.arg(0).as_int().unwrap();
            let v = MyType { number };
            Ok(ctx.new_custom(v))
        }
        let mut vm = Vm::default().with_native_function("custom-function", custom_function);
        assert_eq!(
            vm.eval_str("(custom-function 6)")
                .unwrap()
                .as_custom::<MyType>(),
            Some(&MyType { number: 6 })
        );
    }
}
