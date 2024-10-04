#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Symbol {
    pub(crate) vm_id: u16,
    pub(crate) idx: u32,
}

impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}
