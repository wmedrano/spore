#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub(crate) vm_id: u16,
    pub(crate) idx: u32,
}
