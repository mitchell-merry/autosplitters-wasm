use crate::settings::{ChessPieceSetting, Settings};
use bytemuck::CheckedBitPattern;
use std::collections::HashSet;

// these names come from code directly
#[derive(CheckedBitPattern, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
#[allow(dead_code, non_camel_case_types)]
pub enum Levels {
    #[default]
    /// this one is mine
    Unknown = 69,

    Test = 1,
    FlyingTest = 1429603775,
    Tutorial = 0,
    Pirate = 2,
    Bat = 3,
    Train = 5,
    Veggies = 6,
    Frogs = 7,
    Bee = 1429976377,
    Mouse = 1430652919,
    Dragon = 1432722919,
    Flower = 1450266910,
    Slime = 1450863107,
    Baroness = 1451300935,
    AirshipJelly = 8,
    AirshipStork = 1459338579,
    AirshipCrab = 1459489001,
    FlyingBird = 1428495827,
    FlyingMermaid = 1446558823,
    FlyingBlimp = 1449745424,
    Robot = 1452935394,
    Clown = 1456125457,
    SallyStagePlay = 1456740288,
    DicePalaceDomino = 1458062114,
    DicePalaceCard = 1458289179,
    DicePalaceChips = 1458336090,
    DicePalaceCigar = 1458551456,
    DicePalaceTest = 1458559869,
    DicePalaceBooze = 1458719430,
    DicePalaceRoulette = 1459105708,
    DicePalacePachinko = 1459444983,
    DicePalaceRabbit = 1459928905,
    AirshipClam = 1459950766,
    FlyingGenie = 1460200177,
    DicePalaceLight = 1463124738,
    DicePalaceFlyingHorse = 1463479514,
    DicePalaceFlyingMemory = 1464322003,
    DicePalaceMain = 1465296077,
    DicePalaceEightBall = 1468483834,
    Devil = 1466688317,
    RetroArcade = 1469187579,
    Mausoleum = 1481199742,
    House = 1484633053,
    DiceGate = 1495090481,
    ShmupTutorial = 1504847973,
    Airplane = 1511943573,
    RumRunners = 1518081307,
    OldMan = 1523429320,
    ChessBishop = 1526556188,
    SnowCult = 1527591209,
    FlyingCowboy = 1530096313,
    TowerOfPower = 1553597811,
    ChessBOldA = 1557479427,
    ChessKnight = 1560339521,
    ChessRook = 1560855325,
    ChessQueen = 1561124831,
    ChessPawn = 1562078899,
    ChessKing = 1562579243,
    Kitchen = 1566994171,
    ChessBOldB = 1571650861,
    Saltbaker = 1573044456,
    ChaliceTutorial = 1580294079,
    /// angel / devil
    Graveyard = 1616405510,
    ChessCastle = 1624358789,
    Platforming_Level_1_1 = 1464969490,
    Platforming_Level_1_2 = 1464969491,
    Platforming_Level_3_1 = 1464969492,
    Platforming_Level_3_2 = 1464969493,
    Platforming_Level_2_2 = 1496818712,
    Platforming_Level_2_1 = 1499704951,
}

impl Levels {
    pub fn split_on_scene_transition_to(&self) -> Option<(&str, HashSet<&str>)> {
        match self {
            Levels::Tutorial => Some((
                "scene_level_tutorial",
                HashSet::from(["scene_level_house_elder_kettle", "scene_map_world_1"]),
            )),
            Levels::ChaliceTutorial => Some((
                "scene_level_chalice_tutorial",
                HashSet::from(["scene_map_world_DLC"]),
            )),
            _ => None,
        }
    }

    pub fn get_type(&self) -> LevelType {
        match self {
            Levels::Tutorial | Levels::ChaliceTutorial => LevelType::Tutorial,
            Levels::Mausoleum => LevelType::Mausoleum,
            // isle 1
            Levels::Veggies
            | Levels::Frogs
            | Levels::Slime
            | Levels::FlyingBlimp
            | Levels::Flower
            // isle 2
            | Levels::Baroness
            | Levels::FlyingBird
            | Levels::FlyingGenie
            | Levels::Clown
            | Levels::Dragon
            // isle 3
            | Levels::Bee
            | Levels::Robot
            | Levels::SallyStagePlay
            | Levels::Mouse
            | Levels::Pirate
            | Levels::FlyingMermaid
            | Levels::Train
            // isle hell
            | Levels::DicePalaceMain
            | Levels::Devil
            // isle dlc
            | Levels::OldMan
            | Levels::SnowCult
            | Levels::Airplane
            | Levels::RumRunners
            | Levels::FlyingCowboy
            | Levels::Graveyard
            // | Levels::Flying
            | Levels::Saltbaker => LevelType::Boss,
            Levels::Platforming_Level_1_1
            | Levels::Platforming_Level_1_2
            | Levels::Platforming_Level_2_1
            | Levels::Platforming_Level_2_2
            | Levels::Platforming_Level_3_1
            | Levels::Platforming_Level_3_2 => LevelType::Platformer,
            Levels::ChessPawn
            | Levels::ChessKnight
            | Levels::ChessBishop
            | Levels::ChessRook
            | Levels::ChessQueen => LevelType::ChessPiece,
            _ => LevelType::Unknown,
        }
    }

    pub fn is_split_enabled(&self, settings: &Settings) -> bool {
        match self.get_type() {
            LevelType::Tutorial => settings.split_tutorial,
            LevelType::Mausoleum => settings.split_mausoleum_completion,
            LevelType::Platformer | LevelType::Boss => settings.split_boss_completion,
            LevelType::ChessPiece => match settings.split_chess {
                ChessPieceSetting::EachPiece => true,
                ChessPieceSetting::Never => false,
                ChessPieceSetting::GauntletOnly => *self == Levels::ChessQueen,
            },
            _ => false,
        }
    }
}

#[derive(Default, PartialEq, Eq)]
pub enum LevelType {
    #[default]
    Unknown,
    Tutorial,
    Boss,
    Platformer,
    Mausoleum,
    ChessPiece,
}
