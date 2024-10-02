use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

#[allow(unused_imports)]
use log::*;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum CustomValError {
    #[error("Lock is poisoned")]
    LockPoisoned,
    #[error("Lock is not available")]
    LockNotAvailable,
    #[error("expected type {expected} but found type {actual}")]
    WrongType {
        expected: &'static str,
        actual: &'static str,
    },
}

impl<T> From<TryLockError<T>> for CustomValError {
    fn from(value: TryLockError<T>) -> Self {
        match value {
            // Untested OK: It hard to recreate poisoning a lock while also not invalidating the
            // entire VM. Not terribly bad not to test as the scenario will rarely matter.
            TryLockError::Poisoned(_) => CustomValError::LockPoisoned,
            TryLockError::WouldBlock => CustomValError::LockNotAvailable,
        }
    }
}

/// Contains a custom value.
#[derive(Debug)]
pub struct CustomVal(pub(crate) RwLock<Box<dyn CustomTypeSealed>>);

impl CustomVal {
    /// Create a new `CustomVal` from any type that implements `CustomType`.
    pub fn new(val: impl CustomType) -> CustomVal {
        CustomVal(RwLock::new(Box::new(val)))
    }

    /// Get the underlying value of [Self] if it is of type `T` or else return `None`.
    ///
    /// # Panic
    /// Panics if the value if the value is under a [Self::get_mut].
    pub fn get<T>(&self) -> Result<CustomValRef<'_, T>, CustomValError>
    where
        T: CustomType,
    {
        let guard = self.0.try_read()?;
        let want_type_id = std::any::TypeId::of::<T>();
        let have_type_id = guard.as_any().type_id();

        if want_type_id == have_type_id {
            Ok(CustomValRef {
                guard,
                _type: PhantomData,
            })
        } else {
            Err(CustomValError::WrongType {
                expected: std::any::type_name::<T>(),
                actual: guard.name(),
            })
        }
    }

    /// Get the underlying value of [Self] if it is of type `T` or else return `None`.
    ///
    /// # Panic
    /// Panics if the value if the value is under a [Self::get] or [Self::get_mut].
    pub fn get_mut<T>(&self) -> Result<CustomValMut<T>, CustomValError>
    where
        T: CustomType,
    {
        let guard = self.0.try_write()?;
        let want_type_id = std::any::TypeId::of::<T>();
        let have_type_id = guard.as_any().type_id();
        if want_type_id == have_type_id {
            Ok(CustomValMut {
                guard,
                _type: PhantomData,
            })
        } else {
            Err(CustomValError::WrongType {
                expected: std::any::type_name::<T>(),
                actual: guard.name(),
            })
        }
    }
}

impl std::fmt::Display for CustomVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.try_read() {
            Err(_) => write!(f, "<custom-value-read-locked>"),
            Ok(obj) => obj.deref().fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct CustomValRef<'a, T> {
    guard: RwLockReadGuard<'a, Box<dyn CustomTypeSealed>>,
    _type: PhantomData<T>,
}

impl<'a, T: 'static> Deref for CustomValRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().as_any().downcast_ref::<T>().unwrap()
    }
}

#[derive(Debug)]
pub struct CustomValMut<'a, T> {
    guard: RwLockWriteGuard<'a, Box<dyn CustomTypeSealed>>,
    _type: PhantomData<T>,
}

impl<'a, T: 'static> Deref for CustomValMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().as_any().downcast_ref::<T>().unwrap()
    }
}

impl<'a, T: 'static> DerefMut for CustomValMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard
            .as_mut()
            .as_any_mut()
            .downcast_mut::<T>()
            .unwrap()
    }
}

pub(crate) trait CustomTypeSealed:
    'static + Send + Sync + std::fmt::Display + std::fmt::Debug
{
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any(&self) -> &dyn Any;
    fn name(&self) -> &'static str {
        std::any::type_name_of_val(self)
    }
}

/// A trait that defines a value that can be created or referenced within the VM.
///
/// ```rust
/// #[derive(Debug, Default)]
/// pub struct MyType(i64);
/// impl spore_vm::val::CustomType for MyType {}
/// impl std::fmt::Display for MyType {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "my number is {}", self.0)
///     }
/// }
/// ```
pub trait CustomType:
    'static + Send + Sync + std::fmt::Display + std::fmt::Debug + std::any::Any
{
}

impl<T> CustomTypeSealed for T
where
    T: CustomType,
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        val::{NativeFunctionContext, UnsafeVal, Val, ValBuilder},
        Vm, VmResult,
    };

    #[derive(Debug, PartialEq)]
    struct MyType {
        number: i64,
    }

    impl CustomType for MyType {}

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
        let val = vm.eval_str("custom-value").unwrap();
        let got = val.try_custom::<MyType>().unwrap();
        assert_eq!(got.deref(), &MyType { number: 42 });
    }

    #[test]
    fn custom_type_can_be_accessed_multiple_times() {
        let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
        vm.eval_str("custom-value").unwrap().map(|vm, val1| {
            let got1 = val1.try_custom::<MyType>().unwrap();
            let val2 = vm.eval_str("custom-value").unwrap();
            let got2 = val2.try_custom::<MyType>().unwrap();
            assert_eq!(got1.deref() as *const MyType, got2.deref() as *const MyType);
        });
    }

    #[test]
    fn custom_type_get_after_get_mut_fails() {
        let mut vm = std::thread::spawn(|| {
            let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
            let val1 = vm.eval_str("custom-value").unwrap();
            let get1 = val1.try_custom_mut::<MyType>().unwrap();
            std::mem::forget(get1);
            std::mem::forget(val1);
            vm
        })
        .join()
        .unwrap();
        let val1 = vm.eval_str("custom-value").unwrap();
        assert_eq!(
            val1.try_custom::<MyType>().unwrap_err(),
            CustomValError::LockNotAvailable
        );
    }

    #[test]
    fn display_on_locked_value_does_not_freeze_or_panic() {
        let mut vm = std::thread::spawn(|| {
            let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
            let val1 = vm.eval_str("custom-value").unwrap();
            let get1 = val1.try_custom_mut::<MyType>().unwrap();
            std::mem::forget(get1);
            std::mem::forget(val1);
            vm
        })
        .join()
        .unwrap();
        let val1 = vm.eval_str("custom-value").unwrap();
        assert_eq!(
            val1.try_custom::<MyType>().unwrap_err(),
            CustomValError::LockNotAvailable
        );
        assert_eq!(val1.to_string(), "<custom-value-read-locked>");
    }

    #[test]
    fn custom_type_get_mut_after_read_fails() {
        let mut vm = std::thread::spawn(|| {
            let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
            let val1 = vm.eval_str("custom-value").unwrap();
            let get1 = val1.try_custom::<MyType>().unwrap();
            let get2 = val1.try_custom::<MyType>().unwrap();
            std::mem::forget(get1);
            std::mem::forget(get2);
            std::mem::forget(val1);
            vm
        })
        .join()
        .unwrap();
        let val1 = vm.eval_str("custom-value").unwrap();
        assert_eq!(
            val1.try_custom_mut::<MyType>().unwrap_err(),
            CustomValError::LockNotAvailable
        );
    }

    #[test]
    fn get_mut_can_mutate_value() {
        let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
        {
            let mutate_val = vm.eval_str("custom-value").unwrap();
            let mut mutate_val_ref = mutate_val.try_custom_mut::<MyType>().unwrap();
            *mutate_val_ref = MyType { number: -42 };
            assert_eq!(mutate_val_ref.deref(), &MyType { number: -42 });
        }

        let val = vm.eval_str("custom-value").unwrap();
        assert_eq!(
            val.try_custom::<MyType>().unwrap().deref(),
            &MyType { number: -42 }
        );
    }

    #[derive(Debug, PartialEq)]
    struct OtherType {
        string: &'static str,
    }

    impl CustomType for OtherType {}

    impl std::fmt::Display for OtherType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "magic string {}", self.string)
        }
    }

    #[test]
    fn custom_type_get_and_get_mut_with_wrong_custom_type_fails() {
        let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
        let val = vm.eval_str("custom-value").unwrap();
        assert_eq!(
            val.try_custom::<OtherType>().unwrap_err(),
            CustomValError::WrongType {
                expected: OtherType { string: "" }.name(),
                actual: MyType { number: 0 }.name(),
            }
        );
        assert_eq!(
            val.try_custom_mut::<OtherType>().unwrap_err(),
            CustomValError::WrongType {
                expected: OtherType { string: "" }.name(),
                actual: MyType { number: 0 }.name(),
            }
        );
    }

    #[test]
    fn custom_type_get_and_get_mut_with_wrong_val_type_fails() {
        let mut vm = Vm::default().with_custom_value("custom-value", MyType { number: 42 });
        let val = vm.eval_str("42.0").unwrap();
        assert_eq!(
            val.try_custom::<OtherType>().unwrap_err(),
            CustomValError::WrongType {
                expected: OtherType { string: "" }.name(),
                actual: UnsafeVal::FLOAT_TYPE_NAME,
            }
        );
        assert_eq!(
            val.try_custom_mut::<OtherType>().unwrap_err(),
            CustomValError::WrongType {
                expected: OtherType { string: "" }.name(),
                actual: UnsafeVal::FLOAT_TYPE_NAME,
            }
        );
    }

    #[test]
    fn custom_type_can_be_made_from_native_function() {
        fn custom_function<'a>(
            ctx: NativeFunctionContext<'a>,
            args: &[Val],
        ) -> VmResult<ValBuilder<'a>> {
            let number = args[0].try_int().unwrap();
            let v = MyType { number };
            Ok(ctx.new_custom(v))
        }
        let mut vm = Vm::default().with_native_function("custom-function", custom_function);
        assert_eq!(
            vm.eval_str("(custom-function 6)")
                .unwrap()
                .try_custom::<MyType>()
                .unwrap()
                .deref(),
            &MyType { number: 6 }
        );
    }

    #[test]
    fn custom_type_has_name() {
        assert_eq!(
            MyType { number: 0 }.name(),
            "spore_vm::val::custom::tests::MyType"
        );
        assert_eq!(
            OtherType { string: "" }.name(),
            "spore_vm::val::custom::tests::OtherType"
        );
    }

    #[test]
    fn hacks_for_code_coverage() {
        // Other type is made solely for a few tests.
        let mut other_type = OtherType { string: "" };
        other_type.as_any();
        other_type.as_any_mut();
        other_type.to_string();
    }
}
