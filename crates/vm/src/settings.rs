/// Settings for the Spore virtual machine.
#[derive(Copy, Clone, Debug, Default)]
pub struct Settings {
    /// If aggressive inlining should be used. This should be disabled for any interactive
    /// development where values may be redefined.
    pub enable_aggressive_inline: bool,
}
