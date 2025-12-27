#[macro_use]
extern crate helpers;
use asr::settings::gui::Title;
use asr::settings::Gui;
use asr::{future::next_tick, timer, watcher::Watcher, Error, Process};
use std::collections::HashSet;
use zdoom::pclass::PClass;
use zdoom::{player::DVector3, GameAction, ZDoom, ZDoomVersion};

use helpers::{impl_auto_splitter_state, split};

asr::async_main!(stable);

const TRUE_ENDING_POSITION: DVector3 = DVector3 {
    x: -15032.0,
    y: 9752.0,
    z: -632.0,
};

#[derive(Gui)]
struct Settings {
    /// Split on run end
    _item_noheartbeat: bool,
    /// Split on entering level
    #[heading_level = 0]
    split_entering_level: Title,
    /// Mansion Upstairs
    _level_map02: bool,
    /// Mansion Attic
    _level_map03: bool,
    /// Underground (cave outside mansion)
    _level_map05: bool,
    /// Even more underground (where the gas mask is)
    _level_map08: bool,
    /// Toxic Room
    _level_map06: bool,
    /// Second part of the outside
    _level_map10: bool,
    /// Outside the Cemetery (leaving the cemetery out the back)
    _level_map07: bool,
    /// Split on item pickup
    #[heading_level = 0]
    split_item_pickup: Title,
    /// Story Items
    #[heading_level = 1]
    split_item_pickup_story: Title,
    /// Torture Room Key (basement 3)
    _item_keybasement3: bool,
    /// The next key (basement 2)
    _item_keybasement2: bool,
    /// Bolt Cutter
    _item_boltcutter: bool,
    /// Another basement key (basement 1)
    _item_keybasement1: bool,
    /// Key just after the timer puzzle (11)
    _item_keyf11: bool,
    /// Key just after the key just after the timer puzzle (12)
    _item_keyf12: bool,
    /// Key just after the key just after the key just after the timer puzzle (13)
    _item_keyf13: bool,
    /// Key in the library (22)
    _item_keyf22: bool,
    /// Bomb
    _item_plasticbomb: bool,
    /// Key from statue puzzle (21)
    _item_keyf21: bool,
    /// Yellow Cable (idk on a table)
    _item_yellowcable: bool,
    /// Rooftop Key (14)
    _item_keyf14: bool,
    /// Red Cable (in cage puzzle)
    _item_redcable: bool,
    /// Ruby (in outside statue)
    _item_ruby: bool,
    /// Key near the Girl (15)
    _item_keyf15: bool,
    /// Emerald (in party)
    _item_emerald: bool,
    /// Crank
    _item_squarecrank: bool,
    /// Gas Mask
    _item_gazmask: bool,
    /// Topaz (in gas mask area)
    _item_topaz: bool,
    /// Cemetery Key
    _item_keycemetery: bool,
    /// Shovel
    _item_shovel: bool,
    /// Statue Head
    _item_helenahead: bool,
    /// Coin (from statue)
    _item_dm_coin: bool,
    /// Key from cemetery attic (H3)
    _item_dm_keyfh3: bool,
    /// Key from cemetery morgue (H2)
    _item_dm_keyfh2: bool,
    /// Weapons
    #[heading_level = 1]
    split_item_pickup_weapons: Title,
    /// Fireaxe
    _item_fireaxe: bool,
    /// Beretta
    _item_beretta: bool,
    /// Shotgun
    _item_dm_shotgun: bool,
    /// Annihilator
    _item_annihilator: bool,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("lzdoom.exe").await;
        process
            .until_closes(async {
                on_attach(&process, &mut settings).await.expect("problem");
            })
            .await;
    }
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Option<Error>> {
    let (mut zdoom, _) =
        ZDoom::wait_try_load(process, ZDoomVersion::Lzdoom3_82, "lzdoom.exe", |_| Ok(())).await;

    // zdoom.dump();

    let mut watchers = Watchers::default();
    let mut completed_splits = HashSet::new();

    loop {
        if !process.is_open() {
            asr::print_message("process not open");
            return Ok(());
        }

        settings.update();

        let res = watchers.update(process, &mut zdoom);
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

        if timer::state() == timer::TimerState::NotRunning
            && old.level == "map45"
            && current.level == "MAP01"
        {
            completed_splits = HashSet::new();
            timer::start();
        }

        if timer::state() == timer::TimerState::Running {
            match current.gameaction {
                GameAction::WorldDone => timer::pause_game_time(),
                _ => timer::resume_game_time(),
            }

            if old.level != current.level {
                let key = &format!("_level_{}", current.level.to_lowercase());
                split(key, &mut completed_splits);
            }

            if !old.inventories.is_empty() {
                for inventory in current.inventories {
                    if !old.inventories.contains(&inventory) {
                        asr::print_message(&format!("Picked up {inventory}"));
                        let key = &format!("_item_{}", inventory.to_owned().to_lowercase());
                        split(key, &mut completed_splits);
                    }
                }
            }

            if current.level == "MAP07"
                && current.player_pos == TRUE_ENDING_POSITION
                && old.player_pos != current.player_pos
            {
                split(&String::from("split_run_end"), &mut completed_splits);
            }
        }

        next_tick().await;
    }
}

impl_auto_splitter_state!(Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    player_pos: Watcher<DVector3>,
    inventories: Watcher<HashSet<String>>,
});

impl Watchers {
    fn update(&mut self, process: &Process, zdoom: &mut ZDoom) -> Result<(), Option<Error>> {
        zdoom.invalidate_cache().expect("");

        let gameaction = zdoom.gameaction().unwrap_or_default();
        self.gameaction.update(Some(gameaction));

        let level_name = zdoom.level.name().map(|s| s.to_owned()).unwrap_or_default();
        self.level.update(Some(level_name));

        let player = zdoom.player()?;
        let player_pos = player.pos().map(|v| v.to_owned()).unwrap_or_default();
        self.player_pos.update(Some(player_pos));

        let mut inventories = HashSet::new();
        let invs = zdoom.player()?.get_inventories().unwrap_or_default();

        for inv in invs {
            let class = process.read::<u64>(inv + 0x8)?.into();
            let class = PClass::new(
                process,
                zdoom.memory.clone(),
                zdoom.name_data.clone(),
                class,
            );

            let name = class.name()?;
            inventories.insert(name.to_owned());
        }
        let mut vec = Vec::from_iter(inventories.clone());
        vec.sort();
        self.inventories.update(Some(inventories));

        Ok(())
    }
}
