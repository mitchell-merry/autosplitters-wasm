use asr::string::ArrayWString;
use helpers::watchers::unity::UnityPointerPath;
use helpers::watchers::{ValueGetter, Watcher};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::error::Error;
use std::str::FromStr;
use strum::{Display, EnumString};

#[derive(EnumString, Clone, Display, PartialEq, Eq, Debug, Hash)]
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

    #[strum(default, to_string = "unknown scene ({0})")]
    Unknown(String),
}

static ISLE_ONE_START_FROM: Lazy<HashSet<Scene>> =
    Lazy::new(|| HashSet::from([Scene::LevelElderKettle, Scene::LevelTutorial]));

static ISLE_TWO_START_FROM: Lazy<HashSet<Scene>> = Lazy::new(|| HashSet::from([Scene::IsleOne]));

static ISLE_THREE_START_FROM: Lazy<HashSet<Scene>> = Lazy::new(|| HashSet::from([Scene::IsleTwo]));

static ISLE_HELL_START_FROM: Lazy<HashSet<Scene>> = Lazy::new(|| HashSet::from([Scene::IsleThree]));

static ISLE_DLC_START_FROM: Lazy<HashSet<Scene>> =
    Lazy::new(|| HashSet::from([Scene::IsleOne, Scene::IsleTwo, Scene::IsleThree]));

impl Scene {
    pub fn isle_start_on_scene_transition_from(&self) -> Option<&'static HashSet<Scene>> {
        match self {
            Scene::IsleOne => Some(&ISLE_ONE_START_FROM),
            Scene::IsleTwo => Some(&ISLE_TWO_START_FROM),
            Scene::IsleThree => Some(&ISLE_THREE_START_FROM),
            Scene::IsleHell => Some(&ISLE_HELL_START_FROM),
            Scene::IsleDLC => Some(&ISLE_DLC_START_FROM),
            _ => None,
        }
    }
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
