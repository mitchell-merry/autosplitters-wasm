use crate::enums::Levels;
use asr::settings::gui::Title;
use asr::settings::Gui;
// Note for doc comments - the first line in a /// comment is the name of the setting / value of the choice
// The text after the double newline is the description, usually visible in a tooltip on hover

#[derive(Gui, PartialEq, Eq)]
pub enum LevelCompleteSetting {
    /// Split on knockout.
    ///
    /// Usually when the "KNOCKOUT!" text appears on screen, as soon as the boss is dead.
    #[default]
    OnKnockout,

    /// Split after the scorecard screen (except Devil/Saltbaker).
    ///
    /// It can be useful to split after the scorecard since it varies depending on what you do in
    ///   the fight (parries, health, star skip)
    AfterScorecard,

    /// Split after the scorecard screen (except Devil only).
    ///
    /// Like after scorecard, but *also* splits after scorecard for Saltbaker. Useful for runs
    /// that continue after saltbaker.
    AfterScorecardIncludingSaltbaker,
}

impl LevelCompleteSetting {
    pub fn should_split_on_knockout(&self, level: Levels) -> bool {
        // devil: runs end on devil
        // mausoleum: no scorecard
        // saltbaker: only if the run stops at saltbaker

        match self {
            LevelCompleteSetting::OnKnockout => true,
            LevelCompleteSetting::AfterScorecard => {
                level == Levels::Devil || level == Levels::Mausoleum || level == Levels::Saltbaker
            }
            LevelCompleteSetting::AfterScorecardIncludingSaltbaker => {
                level == Levels::Devil || level == Levels::Mausoleum
            }
        }
    }
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

    /// Choose when to split
    #[heading_level = 0]
    _split_level_type: Title,

    /// Split on boss + level completions
    #[default = true]
    pub split_boss_completion: bool,

    /// Split on mausoleums
    #[default = false]
    pub split_mausoleum_completion: bool,

    /// Split on tutorial completes
    ///
    /// This includes the normal tutorial and Chalice's tutorial, but not the plane one.
    /// Nobody cares about the plane one.
    #[default = false]
    pub split_tutorial: bool,
}
