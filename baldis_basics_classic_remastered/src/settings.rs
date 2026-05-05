use asr::settings::Gui;

// Note for doc comments - the first line in a /// comment is the name of the setting / value of the choice
// The text after the double newline is the description, usually visible in a tooltip on hover

#[derive(Gui)]
pub struct Settings {
    /// some setting
    ///
    /// some description
    pub individual_level_mode: bool,
}
