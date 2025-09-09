use asr::settings::gui::Title;
use asr::settings::Gui;

#[derive(Gui, PartialEq, Eq)]
pub enum LevelCompleteSetting {
    /// Don't split on level complete.
    NoSplit = 0,
    /// Split on knockout.
    ///
    /// When the "KNOCKOUT!" text appears on screen, as soon as the boss is dead.
    #[default]
    OnKnockout = 1,
    /// Split after the scorecard screen.
    ///
    /// It can be useful to split after the scorecard since it varies depending on what you do in
    ///   the fight (parries, health, star skip)
    AfterScorecard = 2,
}

#[derive(Gui)]
pub struct Settings {
    /// Choose whether to enable individual level mode
    pub individual_level_mode: bool,
    /// Choose how to split on level complete (ignored when individual level mode is on)j
    pub split_level_complete: LevelCompleteSetting,

    /// Split on entering level
    #[heading_level = 0]
    split_entering_level: Title,
    /// Mansion Upstairs
    _level_map02: bool,
}
