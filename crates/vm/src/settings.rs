/// Settings for the Spore virtual machine.
#[derive(Copy, Clone, Debug)]
pub struct Settings {
    /// If aggressive inlining should be used. This should be disabled for any interactive
    /// development where values may be redefined.
    pub enable_aggressive_inline: bool,
    /// If true, debug information will be preserved at the cost of higher RAM usage.
    pub enable_source_maps: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            enable_aggressive_inline: false,
            enable_source_maps: true,
        }
    }
}
