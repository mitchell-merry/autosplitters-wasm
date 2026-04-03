use asr::string::ArrayWString;
use helpers::watchers::unity::UnityPointerPath;
use helpers::watchers::{ValueGetter, Watcher};
use std::error::Error;
use std::str::FromStr;
use strum::{Display, EnumString};

#[derive(EnumString, Copy, Clone, Default, Display, PartialEq, Eq, Debug, Hash)]
pub enum Scene {
    #[strum(serialize = "scene_title")]
    TitleScreen,

    #[strum(serialize = "scene_cutscene_intro")]
    CutsceneIntro,

    #[strum(serialize = "scene_map_world_1")]
    IsleOne,

    #[strum(serialize = "scene_map_world_2")]
    IsleTwo,

    #[strum(serialize = "scene_map_world_3")]
    IsleThree,

    #[strum(serialize = "scene_map_world_4")]
    IsleHell,

    #[strum(serialize = "scene_map_world_DLC")]
    IsleDLC,

    #[strum(serialize = "scene_cutscene_kingdice")]
    CutsceneKingDice,

    #[strum(serialize = "scene_cutscene_devil")]
    CutsceneDevil,

    #[strum(serialize = "scene_win")]
    Scoreboard,

    #[strum(serialize = "scene_level_house_elder_kettle")]
    LevelElderKettle,

    #[strum(serialize = "scene_level_tutorial")]
    LevelTutorial,

    #[strum(serialize = "scene_level_chalice_tutorial")]
    LevelChaliceTutorial,

    #[strum(serialize = "scene_level_mausoleum")]
    LevelMausoleum,

    #[strum(serialize = "scene_level_graveyard")]
    LevelGraveyard,

    #[strum(serialize = "scene_level_chess_castle")]
    LevelChessCastle,

    #[strum(serialize = "scene_level_chess_pawn")]
    LevelChessPawn,

    #[strum(serialize = "scene_level_chess_knight")]
    LevelChessKnight,

    #[strum(serialize = "scene_level_chess_bishop")]
    LevelChessBishop,

    #[strum(serialize = "scene_level_chess_rook")]
    LevelChessRook,

    #[strum(serialize = "scene_level_chess_queen")]
    LevelChessQueen,

    #[default]
    #[strum(to_string = "unknown scene")]
    Unknown,
}

pub struct SceneGetter<'a> {
    scene_str_getter: Box<dyn ValueGetter<ArrayWString<128>> + 'a>,
}

impl<'a> SceneGetter<'a> {
    pub fn new(scene_str_getter: Box<dyn ValueGetter<ArrayWString<128>> + 'a>) -> Self {
        SceneGetter { scene_str_getter }
    }
}

impl<'a> From<UnityPointerPath<'a>> for SceneGetter<'a> {
    fn from(value: UnityPointerPath<'a>) -> Self {
        SceneGetter::new(Box::new(value))
    }
}

impl<'a> ValueGetter<Scene> for SceneGetter<'a> {
    fn get(&self) -> Result<Scene, Box<dyn Error>> {
        let r = self.scene_str_getter.get()?;
        let string = String::from_utf16(r.as_slice())?;

        Ok(Scene::from_str(&string)?)
    }
}

impl<'a> From<SceneGetter<'a>> for Watcher<'a, Scene> {
    fn from(value: SceneGetter<'a>) -> Self {
        Watcher::new(Box::new(value))
    }
}
