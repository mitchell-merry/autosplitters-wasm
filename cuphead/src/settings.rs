use asr::settings::gui::Title;
use asr::settings::Gui;

#[derive(Gui, PartialEq, Eq)]
pub enum LevelCompleteSetting {
    /// Split on knockout.
    ///
    /// Usually when the "KNOCKOUT!" text appears on screen, as soon as the boss is dead.
    #[default]
    OnKnockout,
    /// Split after the scorecard screen.
    ///
    /// It can be useful to split after the scorecard since it varies depending on what you do in
    ///   the fight (parries, health, star skip)
    AfterScorecard,
}

#[derive(Gui)]
pub struct Settings {
    /// Individual Level Mode
    ///
    /// Use in-game-time, start time on each level attempt, reset when a level is reset or is left.
    pub individual_level_mode: bool,
    /// Choose how to split on level complete (ignored when individual level mode is on)
    ///
    /// This only matters for levels which have a scorecard.
    pub split_level_complete: LevelCompleteSetting,

    /// Choose whether to split on each level type
    #[heading_level = 0]
    _split_level_type: Title,
    /// Split on boss completions
    #[default = true]
    pub split_boss_completion: bool,
    /// Split on mausoleums
    #[default = true]
    pub split_mausoleum_completion: bool,
    /// Split on tutorial completes
    #[default = true]
    pub split_tutorial: bool,
}
