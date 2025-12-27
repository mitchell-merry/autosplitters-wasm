#[macro_use]
extern crate helpers;
use asr::settings::gui::Title;
use asr::settings::Gui;
use asr::string::ArrayCString;
use asr::time::Duration;
use asr::{future::next_tick, timer, watcher::Watcher, Address, Error, Process};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use zdoom::pclass::PClass;
use zdoom::tarray::TArray;
use zdoom::{
    player::{DVector3, PlayerState},
    GameAction, ZDoom, ZDoomVersion,
};

use helpers::{impl_auto_splitter_state, split};

asr::async_main!(stable);

#[derive(Gui)]
struct Settings {
    #[default = true]
    /// Split on the results screen (run end)
    split_on_results: bool,
    objectives: Title,
    /// Preparations
    #[heading_level = 1]
    _999: Title,
    /// Find a way past the soldiers and locate a weapon
    _4294966296: bool,
    /// Prepare to fight
    _4294966295: bool,
    /// Locate your combat suit from Personal Belongings
    _4294966294: bool,
    /// The Lockdown
    #[heading_level = 1]
    _995: Title,
    /// Head back to the Blue Door
    _4294966292: bool,
    /// Find a way to disengage the lockdown
    _4294966293: bool,
    #[heading_level = 1]
    /// Escape
    _3: Title,
    /// Defeat the Juggernaut
    _4294966290: bool,
    /// Find the exit.
    _4294966291: bool,
    #[heading_level = 1]
    /// A Way Out
    _200: Title,
    /// Reach the Rooftops
    _201: bool,
    /// Find a way into the Orange Wing of Pathfinder Memorial
    _202: bool,
    /// Descend to the Sky Lounge
    _203: bool,
    #[heading_level = 1]
    /// Safety Hazard
    _220: Title,
    /// Find a way to de-electrify the water
    _221: bool,
    #[heading_level = 1]
    /// Detour
    _210: Title,
    /// Push through the Utility Area
    _212: bool,
    /// Reach Water Treatment Plant
    _213: bool,
    /// Find a way inside 'The Broken Seal' bar
    _215: bool,
    /// Locate elevator and descend even further
    _216: bool,
    /// Push through the Parking Garage
    _217: bool,
    /// Descend to Street Level
    _211: bool,
    #[heading_level = 1]
    /// Underground Operations
    _300: Title,
    /// Reach Sal's Bar
    _301: bool,
    /// Acquire Demolition Charges in hidden basement floor
    _302: bool,
    /// Expose hidden elevator behind the Wine Rack
    _306: bool,
    /// Head back to ground level
    _304: bool,
    /// Find a way inside Liancria Offices
    _305: bool,
    #[heading_level = 1]
    /// Mall Security (Purple)
    _520: Title,
    /// Enter the South Wing
    _521: bool,
    /// Explore South Wing stores and locate the Purple Key
    _522: bool,
    #[heading_level = 1]
    /// Mall Security (Blue)
    _510: Title,
    /// Enter the North Wing
    _511: bool,
    /// Explore North Wing stores and locate the Blue Key
    _512: bool,
    #[heading_level = 1]
    /// Mall Security
    _500: Title,
    /// Leave the Shopping Mall through the other side of the building.
    _501: bool,
    /// Unlock the exit by activating both Key Card panels.
    _502: bool,
    #[heading_level = 1]
    /// Safety First!
    _600: Title,
    /// Acquire safety equipment
    _601: bool,
    #[heading_level = 1]
    /// Preventive Measures
    _310: Title,
    /// Reach the Cloning Facility in Floor B2
    _318: bool,
    /// Gain access to the Plant Factory
    _311: bool,
    /// Place demo charge on the first target
    _312: bool,
    /// Place demo charge on the second target
    _313: bool,
    /// Place demo charge on the third target
    _314: bool,
    /// Place demo charge on the final target
    _315: bool,
    /// Evacuate to the elevator
    _316: bool,
    #[heading_level = 1]
    /// Starlight (Optional) has status 2
    _700: Title,
    /// Access the Starlight facility
    _701: bool,
    // #[heading_level = 1]
    // /// Endgame
    // _720: Title,
    // /// Push through the tunnels to ACE HQ
    // _722: bool,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("SELACO.exe").await;
        process
            .until_closes(async {
                on_attach(&process, &mut settings).await.expect("problem");
            })
            .await;
    }
}

struct FoundClasses<'a> {
    objectives_class: PClass<'a>,
    objective_class: PClass<'a>,
    _actor_class: PClass<'a>,
}

#[derive(PartialEq)]
enum LevelTransitionLoadState {
    NotTransitioning,
    ActionCompleted,
    AfterCompletedBeforeWorldDone,
    ActionWorldDone,
    ActionAutoSaveAfterWorldDone,
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Option<Error>> {
    let (mut zdoom, classes) = ZDoom::wait_try_load(
        process,
        ZDoomVersion::Gzdoom4_8Pre,
        "Selaco.exe",
        |classes| {
            let objectives_class = classes.get("Objectives").ok_or(None)?.to_owned();
            let objective_class = classes.get("Objective").ok_or(None)?.to_owned();
            let actor_class = classes.get("Actor").ok_or(None)?.to_owned();

            Ok(FoundClasses {
                objectives_class,
                objective_class,
                _actor_class: actor_class,
            })
        },
    )
    .await;
    // let _ = zdoom.dump();
    // let _ = zdoom.level.dump_actors(&classes.actor_class);

    let mut watchers = Watchers::default();
    let mut completed_splits = HashSet::new();
    let mut level_transition_state = LevelTransitionLoadState::NotTransitioning;

    loop {
        if !process.is_open() {
            asr::print_message("process not open");
            return Ok(());
        }

        settings.update();

        let res = watchers.update(process, &mut zdoom, &classes);
        if res.is_err() {
            asr::print_message("failed updating watchers");
            continue;
        }

        let states = watchers.to_states();
        if states.is_none() {
            asr::print_message("some watcher is empty");
            continue;
        }

        let (old, current) = states.unwrap();

        match level_transition_state {
            LevelTransitionLoadState::NotTransitioning => {
                match current.gameaction {
                    GameAction::Completed => {
                        level_transition_state = LevelTransitionLoadState::ActionCompleted
                    }
                    GameAction::WorldDone => {
                        level_transition_state = LevelTransitionLoadState::ActionWorldDone
                    }
                    _ => {}
                };
            }
            LevelTransitionLoadState::ActionCompleted => {
                match current.gameaction {
                    GameAction::Nothing => {
                        level_transition_state =
                            LevelTransitionLoadState::AfterCompletedBeforeWorldDone
                    }
                    GameAction::WorldDone => {
                        level_transition_state = LevelTransitionLoadState::ActionWorldDone
                    }
                    _ => {}
                };
            }
            LevelTransitionLoadState::AfterCompletedBeforeWorldDone => {
                if current.gameaction == GameAction::WorldDone {
                    level_transition_state = LevelTransitionLoadState::ActionWorldDone;
                }
            }
            LevelTransitionLoadState::ActionWorldDone => match current.gameaction {
                GameAction::AutoSave => {
                    level_transition_state = LevelTransitionLoadState::ActionAutoSaveAfterWorldDone
                }
                GameAction::Nothing => {
                    level_transition_state = LevelTransitionLoadState::NotTransitioning
                }
                _ => {}
            },
            LevelTransitionLoadState::ActionAutoSaveAfterWorldDone => {
                if current.gameaction == GameAction::Nothing {
                    level_transition_state = LevelTransitionLoadState::NotTransitioning;
                }
            }
        }

        if timer::state() == timer::TimerState::NotRunning {
            completed_splits = HashSet::new();

            if current.level == "SE_01a"
                && old.gameaction == GameAction::NewGame
                && current.gameaction != GameAction::NewGame
            {
                completed_splits = HashSet::new();
                timer::start();
                asr::timer::set_game_time(Duration::ZERO);
            }
        }

        if timer::state() == timer::TimerState::Running {
            // isLoading
            if level_transition_state != LevelTransitionLoadState::NotTransitioning
                || current.playerstate == PlayerState::Enter
            {
                timer::pause_game_time();
            } else {
                timer::resume_game_time();
            }

            // split
            for (objective_key, old_objective_status) in old.objective_status {
                if let Some(current_objective_status) = current.objective_status.get(&objective_key)
                {
                    // idk what status 2 means but i've seen it
                    if old_objective_status == 0 && *current_objective_status != 0 {
                        asr::print_message(&format!("completed {objective_key}"));

                        split(&objective_key, &mut completed_splits);
                    }
                }
            }

            if settings.split_on_results
                && old.player_pos != current.player_pos
                && current.player_pos.x == -22780.0
                && current.player_pos.y == 8008.0
                && current.player_pos.z == -1448.0
            {
                asr::timer::split();
            }
        }

        // if old.actors.len() < current.actors.len() && old.actors.len() != 0 {
        //     for a in current.actors {
        //         if !old.actors.contains(&a) {
        //             asr::print_message(&format!("new actor {a}"));
        //         }
        //     }
        // }

        next_tick().await;
    }
}

impl_auto_splitter_state!(Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    playerstate: Watcher<PlayerState>,
    player_pos: Watcher<DVector3>,
    objective_status: Watcher<HashMap<String, u32>>,
});

impl Watchers {
    fn update<'a>(
        &mut self,
        process: &'a Process,
        zdoom: &mut ZDoom<'a>,
        classes: &FoundClasses<'a>,
    ) -> Result<(), Option<Error>> {
        zdoom.invalidate_cache().expect("");

        let gameaction = zdoom.gameaction().unwrap_or_default();
        timer::set_variable("gameaction", &format!("{:?}", gameaction));
        self.gameaction.update(Some(gameaction));

        let level_name = zdoom.level.name().map(|s| s.to_owned()).unwrap_or_default();
        timer::set_variable("map", level_name.as_str());
        self.level.update(Some(level_name));

        let player = zdoom.player()?;
        let playerstate = player.state()?.to_owned();
        timer::set_variable("playerstate", &format!("{:?}", playerstate));
        self.playerstate.update(Some(playerstate));

        let player_pos = player.pos().map(|v| v.to_owned()).unwrap_or_default();
        timer::set_variable("pos", &format!("{:?}", player_pos));
        self.player_pos.update(Some(player_pos));

        let (objectives, objective_history) =
            get_completed_objectives(process, zdoom, classes).unwrap_or_default();
        timer::set_variable("objectives", &format!("{:#?}", objectives));
        // timer::set_variable("history", &format!("{:#?}", objective_history));

        let mut map = HashMap::new();
        get_objective_status_map(&objectives, &mut map);
        get_objective_status_map(&objective_history, &mut map);

        // let sorted_map: BTreeMap<_, _> = map.clone().into_iter().collect();
        // timer::set_variable("objective_status", &format!("{:#?}", sorted_map));

        self.objective_status.update(Some(map));

        // let mut actors = zdoom.level.get_actor_names(&classes.actor_class)?;
        // actors.sort();
        // timer::set_variable("actors", &format!("{:#?}", actors));

        Ok(())
    }
}

fn get_objective_status_map(objectives: &Vec<Objective>, map: &mut HashMap<String, u32>) {
    for obj in objectives {
        map.insert(format!("_{}", obj.tag), obj.status);
        get_objective_status_map(&obj.children, map);
    }
}

#[derive(Clone, Debug)]
struct Objective {
    _title: String,
    tag: u32,
    status: u32,
    children: Vec<Objective>,
}

impl Objective {
    pub fn read(
        process: &Process,
        address: Address,
        classes: &FoundClasses,
    ) -> Result<Self, Option<Error>> {
        let children_offset = classes
            .objective_class
            .fields()?
            .get("children")
            .ok_or(None)?
            .offset()?
            .to_owned() as u64;
        let title_offset = classes
            .objective_class
            .fields()?
            .get("title")
            .ok_or(None)?
            .offset()?
            .to_owned() as u64;
        let status_offset = classes
            .objective_class
            .fields()?
            .get("status")
            .ok_or(None)?
            .offset()?
            .to_owned() as u64;
        let tag_offset = classes
            .objective_class
            .fields()?
            .get("tag")
            .ok_or(None)?
            .offset()?
            .to_owned() as u64;

        let title = process
            .read_pointer_path::<ArrayCString<128>>(
                address,
                asr::PointerSize::Bit64,
                &[title_offset, 0x0],
            )?
            .validate_utf8()
            .expect("title should always be utf-8")
            .to_owned();

        let tag = process.read(address + tag_offset)?;
        let status = process.read(address + status_offset)?;
        let children = read_objectives(process, address + children_offset, classes)?;

        Ok(Objective {
            _title: title,
            tag,
            status,
            children,
        })
    }
}

fn read_objectives(
    process: &Process,
    address: Address,
    classes: &FoundClasses,
) -> Result<Vec<Objective>, Option<Error>> {
    let objectives_arr = TArray::new(process, address);
    let mut objectives = Vec::new();
    for objective in objectives_arr.iter::<u64>()? {
        if let Ok(obj) = Objective::read(process, objective.into(), classes) {
            objectives.push(obj);
        }
    }
    Ok(objectives)
}

fn get_completed_objectives(
    process: &Process,
    zdoom: &ZDoom,
    classes: &FoundClasses,
) -> Result<(Vec<Objective>, Vec<Objective>), Option<Error>> {
    let objs_offset = classes
        .objectives_class
        .fields()?
        .get("objs")
        .ok_or(None)?
        .offset()?
        .to_owned() as u64;
    let history_offset = classes
        .objectives_class
        .fields()?
        .get("history")
        .ok_or(None)?
        .offset()?
        .to_owned() as u64;

    let player = zdoom.player()?;
    let player_inventories = player.get_inventories()?;

    for inv in player_inventories {
        let class = process.read::<u64>(inv + 0x8)?.into();
        let class = PClass::new(
            process,
            zdoom.memory.clone(),
            zdoom.name_data.clone(),
            class,
        );

        let name = class.name()?;
        if name == "Objectives" {
            return Ok((
                read_objectives(process, inv + objs_offset, classes)?,
                read_objectives(process, inv + history_offset, classes)?,
            ));
        }
    }

    Ok((Vec::default(), Vec::default()))
}
