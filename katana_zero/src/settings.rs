use asr::settings::Gui;

// Note for doc comments - the first line in a /// comment is the name of the setting / value of the choice
// The text after the double newline is the description, usually visible in a tooltip on hover

#[derive(Gui)]
pub struct Settings {
    /// (Experimental) Pause game time on transitions
    ///
    /// Transitions (such as Dithers) now pause time.
    pub pause_on_transition: bool,
}
