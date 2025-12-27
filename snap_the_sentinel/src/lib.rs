#[macro_use]
extern crate helpers;
use asr::settings::gui::Title;
use asr::settings::Gui;
use asr::{future::next_tick, print_message, timer, watcher::Watcher, Error, Process};
use std::collections::HashSet;
use zdoom::{player::DVector3, GameAction, ZDoom, ZDoomVersion};

use helpers::{impl_auto_splitter_state, split};

asr::async_main!(stable);

#[derive(Gui)]
struct Settings {
    /// Split on level completion
    #[heading_level = 0]
    split_level_complete: Title,
    /// E1M1 - Shabby Pad
    _level_e1m1_e1m2: bool,
    /// E1M2 - Lush Canyon
    _level_e1m2_e1m3: bool,
    /// E1M3 - Torrid Caldera
    _level_e1m3_e1m4: bool,
    /// E1M4 - Crystal Excavation
    _level_e1m4_e1m5: bool,
    /// E1M5 - Mysterious Tunnel
    _level_e1m5_e1m6: bool,
    /// E1M6 - Pacific Port
    _level_e1m6_e1m7: bool,
    /// E1M7 - Freighter Frenzy
    _level_e1m7_e1m8: bool,
    /// E1M8 - Midnight Metro
    _level_e1m8_e1m9: bool,
    /// E1M9 - Reef Skyscraper
    _level_e1m9_e1m10: bool,
    /// E1M10 - Ocean's Helipad (on Comrade Ocean's death)
    #[default = true]
    ocean_death: bool,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("gzdoom.exe").await;
        process
            .until_closes(async {
                on_attach(&process, &mut settings).await.expect("problem");
            })
            .await;
    }
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Error> {
    let (mut zdoom, _) =
        ZDoom::wait_try_load(process, ZDoomVersion::Gzdoom4_8_2, "gzdoom.exe", |_| Ok(())).await;
    // zdoom.dump();
    // if let Ok(p) = zdoom.player() {
    //     p.dump_inventories(&zdoom.name_data);
    // }
    if zdoom
        .level
        .dump_actors(zdoom.classes().unwrap().get("Actor").unwrap())
        .is_err()
    {
        print_message("there was an error, but good luck knowing what it was");
    }

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
            && current.level == "E1M1"
            && current.player_pos.x == 64.0
            && current.player_pos.y == -848.0
            && old.gameaction == GameAction::NewGame
            && current.gameaction == GameAction::Nothing
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
                let key = format!("_level_{}_{}", old.level, current.level).to_lowercase();
                split(&key, &mut completed_splits);
            }

            if let Some(old_health) = old.ocean_health {
                if let Some(current_health) = current.ocean_health {
                    if settings.ocean_death && old_health > 0 && current_health == 0 {
                        split(&String::from("ocean_death"), &mut completed_splits);
                    }
                }
            }
        }

        next_tick().await;
    }
}

pub fn get_ocean_health(process: &Process, zdoom: &mut ZDoom) -> Option<u32> {
    let res: Result<u32, Option<Error>> = (|| {
        if zdoom.level.name()? != "E1M10" {
            return Err(None);
        }

        let ocean = zdoom.level.find_actor("DummyOcean")?;
        let actor_health_offset = zdoom
            .classes()?
            .get("Actor")
            .unwrap()
            .fields()?
            .get("Health")
            .unwrap()
            .offset()?
            .to_owned();

        Ok(process.read::<u32>(ocean + actor_health_offset)?)
    })();

    if let Ok(health) = res {
        return Some(health);
    }

    None
}

impl_auto_splitter_state!(Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    player_pos: Watcher<DVector3>,
    ocean_health: Watcher<Option<u32>>,
});

impl Watchers {
    fn update(&mut self, process: &Process, zdoom: &mut ZDoom) -> Result<(), Option<Error>> {
        zdoom.invalidate_cache().expect("");

        self.gameaction
            .update(Some(zdoom.gameaction().unwrap_or_default()));

        let level_name = zdoom.level.name().map(|s| s.to_owned()).unwrap_or_default();
        self.level.update(Some(level_name));

        let player = zdoom.player()?;
        let player_pos = player.pos().map(|v| v.to_owned()).unwrap_or_default();
        self.player_pos.update(Some(player_pos));

        self.ocean_health
            .update(Some(get_ocean_health(process, zdoom)));

        Ok(())
    }
}
