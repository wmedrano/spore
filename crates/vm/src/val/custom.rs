use std::any::Any;

#[derive(Debug)]
pub struct CustomVal(Box<dyn CustomType>);

impl CustomVal {
    pub fn new(val: Box<impl CustomType>) -> CustomVal {
        CustomVal(val)
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: CustomType,
    {
        self.0.as_any().downcast_ref::<T>()
    }
}

impl std::fmt::Display for CustomVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{inner}", inner = self.0)
    }
}

pub trait CustomType: 'static + std::fmt::Display + std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T> CustomType for T
where
    T: 'static + Sized + std::fmt::Display + std::fmt::Debug,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
