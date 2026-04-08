use asr::settings::Gui;

// Note for doc comments - the first line in a /// comment is the name of the setting / value of the choice
// The text after the double newline is the description, usually visible in a tooltip on hover

#[derive(Gui)]
pub struct Settings {
    /// Individual Level Mode
    ///
    /// Start time on each level attempt, reset when a level is reset or is left.
    pub individual_level_mode: bool,

    /// Use in-game-time instead of real time
    ///
    /// Use the in game time instead of real time. In non-IL mode, it tracks total IGT between runs.
    pub use_in_game_time: bool,
}
