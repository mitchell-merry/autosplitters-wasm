use crate::enums::{LevelType, Levels};
use asr::settings::gui::Title;
use asr::settings::Gui;

// Note for doc comments - the first line in a /// comment is the name of the setting / value of the choice
// The text after the double newline is the description, usually visible in a tooltip on hover

#[derive(Gui, PartialEq, Eq)]
pub enum LevelCompleteSetting {
    /// Split on knockout.
    ///
    /// Usually when the "KNOCKOUT!" text appears on screen, as soon as the boss is dead.
    OnKnockout,

    /// Split after the scorecard screen (except Devil/Saltbaker).
    ///
    /// It can be useful to split after the scorecard since it varies depending on what you do in
    ///   the fight (parries, health, star skip)
    #[default]
    AfterScorecard,

    /// Split after the scorecard screen (except Devil only).
    ///
    /// Like after scorecard, but *also* splits after scorecard for Saltbaker. Useful for runs
    /// that continue after saltbaker.
    AfterScorecardIncludingSaltbaker,
}

impl LevelCompleteSetting {
    pub fn should_split_on_knockout(&self, level: Levels) -> bool {
        match self {
            LevelCompleteSetting::OnKnockout => true,
            // for these, which levels override the setting and split on knockout anyways?
            // devil: runs end on devil
            // mausoleum: no scorecard
            // saltbaker: only if the run stops at saltbaker
            // chess pieces: no scorecard
            // angel/devil: no scorecard
            LevelCompleteSetting::AfterScorecard => {
                level == Levels::Devil
                    || level == Levels::Saltbaker
                    || level == Levels::Graveyard
                    || level == Levels::Mausoleum
                    || level.get_type() == LevelType::ChessPiece
            }
            LevelCompleteSetting::AfterScorecardIncludingSaltbaker => {
                level == Levels::Devil
                    || level == Levels::Graveyard
                    || level == Levels::Mausoleum
                    || level.get_type() == LevelType::ChessPiece
            }
        }
    }
}

#[derive(Gui, PartialEq, Eq)]
pub enum ChessPieceSetting {
    /// On completion of each piece
    #[default]
    EachPiece,

    /// On gauntlet completion (i.e. Queen only)
    GauntletOnly,

    /// Never
    Never,
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

    /// Automatically reset the timer on the Title Screen
    ///
    /// This won't occur after every split has been completed.
    /// Additionally, every new best segment will automatically be saved without asking for confirmation, so tread carefully.
    #[default = false]
    pub auto_reset: bool,

    /// Display Star Skip Counter in decimal notation (half = 0.5, full = 1.0)
    ///
    /// For expert mode, 1 star = 0.33, 2 stars = 0.66, 3 stars = 1.0.
    /// If unchecked, each Partial Star Skipped will add 1 to the counter for each star that was skipped.
    #[default = true]
    pub display_star_skip_counter_as_decimal: bool,

    /// Choose when to split
    #[heading_level = 0]
    _split_level_type: Title,

    /// Split on boss + run'n'gun completions
    #[default = true]
    pub split_boss_completion: bool,

    /// Split on taking the Devil's deal (for Bad Ending)
    ///
    /// You can generally leave this on, since it won't have any affect if you don't take the deal.
    #[default = true]
    pub split_devil_deal: bool,

    /// Split on King Dice contract cutscene (for Simple runs)
    ///
    /// For Simple runs, this is time end. Note you should make sure this is OFF for Regular runs.
    /// It will split on the cutscene before KD (they are the same scene).
    ///
    /// Improvement item for this: https://github.com/mitchell-merry/autosplitters-wasm/issues/9
    #[default = false]
    pub split_kd_contract_cutscene: bool,

    /// Split on mausoleums
    #[default = true]
    pub split_mausoleum_completion: bool,

    /// Split on tutorial completes
    ///
    /// This includes the normal tutorial and Chalice's tutorial, but not the plane one.
    /// Nobody cares about the plane one.
    #[default = false]
    pub split_tutorial: bool,

    /// Split on Highest Grade ONLY
    ///
    /// This is useful for categories such as All S+P Grades
    pub split_highest_grade: bool,

    /// Split on the gauntlet
    pub split_chess: ChessPieceSetting,
}
