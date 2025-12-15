extern crate helpers;
mod memory;

use crate::memory::Watchers;
use asr::emulator::gba::Emulator;
use asr::future::next_tick;
use asr::time::Duration;
use asr::timer::{
    pause_game_time, resume_game_time, set_game_time, set_variable, start, state, TimerState,
};
use bitflags::bitflags;
use bytemuck::{CheckedBitPattern, Pod, Zeroable};
use helpers::settings::initialise_settings;
use helpers::watchers::Watcher;
use helpers::{better_split, get_setting};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Debug;

asr::async_main!(stable);

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Attempting to attach...");

    let settings_defaults = initialise_settings(include_str!("../data/settings.ron"))
        .expect("failed to initialise settings");

    loop {
        let emulator = Emulator::wait_attach().await;
        emulator
            .until_closes(async {
                let x = on_attach(&emulator, &settings_defaults).await;
                if let Err(e) = x {
                    asr::print_message(&format!("{}", e));
                }
            })
            .await;
    }
}

// 3004430

// 0x30045EC important flags

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    #[repr(C)]
    pub struct GameFlags: u32 {
        /// the timer is running (reset to 0 when a level starts)
        const HasStarted = 1 << 1;
        /// the player has completed the level (reset to 0 when a level starts)
        const HasFinished = 1 << 3;

        // The following are unused, just some I happened to figure out

        /// the player has died
        const PlayerIsDead = 1 << 5;

        /// player is holding down 1 level of sprint (but not 2)
        const PlayerIsSprinting = 1 << 12;
        /// player is holding down 2 levels of sprint
        const PlayerIsReallySprinting = 1 << 13;
        /// player is in "start" or "checkpoints" squares and should be healed
        const PlayerInHealer = 1 << 18;

        // Make all other bits "known" (for the purpose of displaying them in debug output)
        const _ = !0;
    }
}

#[derive(CheckedBitPattern, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[repr(u8)]
enum GameMode {
    #[default]
    None = 0,
    Normal = 1,
    Easy = 2,
    Challenge = 3,
    Practice = 5,
    MakeUp = 7,
}

fn get_in_game_time(frames: u32) -> Duration {
    Duration::seconds_f32((frames as f32) / 60_f32)
}

fn flag_just_enabled<'a>(
    flags_watcher: &Watcher<'a, GameFlags>,
    flag: GameFlags,
) -> Result<bool, Box<dyn Error>> {
    let old = match flags_watcher.old() {
        Some(x) => x,
        None => return Ok(false),
    };

    Ok(!old.contains(flag) && flags_watcher.current()?.contains(flag))
}

async fn on_attach(
    emulator: &Emulator,
    settings_defaults: &HashMap<String, bool>,
) -> Result<(), Box<dyn Error>> {
    asr::print_message("Attached!");

    // set_variable(
    //     "ram base (ewram)",
    //     &format!("0x{}", emulator.ram_base.get().unwrap().get(0).unwrap()),
    // );
    // set_variable(
    //     "ram base (iwram)",
    //     &format!("0x{}", emulator.ram_base.get().unwrap().get(1).unwrap()),
    // );

    let mut watchers = Watchers::init(emulator);
    let mut completed_splits = HashSet::new();

    while emulator.is_open() {
        next_tick().await;
        watchers.invalidate();

        set_variable("time (frames)", &format!("{}", watchers.time.current()?));

        let flags = watchers.flags.current()?;
        set_variable("flags", &format!("{:08x}", flags.bits()));
        set_variable("flags (interpreted)", &format!("{:?}", flags));

        set_variable("world", &format!("{}", watchers.world.current()?));
        set_variable("sub level", &format!("{:?}", watchers.sub_level.current()?));
        set_variable(
            "current mode",
            &format!("{:?}", watchers.game_mode.current()?),
        );

        set_variable("input", &format!("{:?}", watchers.input_flags.current()?));
        set_variable("state", &format!("{:?}", watchers.state.current()?));
        set_variable("substate", &format!("{:?}", watchers.substate.current()?));

        if state() == TimerState::NotRunning {
            completed_splits.clear();
            if should_start(&watchers, settings_defaults)? {
                start();
            }
        }

        if state() == TimerState::Running {
            if get_setting("igt_mode", &settings_defaults)? {
                set_game_time(get_in_game_time(watchers.time.current()?));
                pause_game_time();
            } else {
                resume_game_time();
            }
        }

        // Splits (level completion)
        if flag_just_enabled(&watchers.flags, GameFlags::HasFinished)? {
            let key = &format!(
                "_level_{:?}_{}",
                watchers.world.current()?,
                watchers.sub_level.current()?
            );
            let _ = better_split(key, settings_defaults, &mut completed_splits);
        }
    }

    Ok(())
}

fn should_start(
    watchers: &Watchers,
    settings_defaults: &HashMap<String, bool>,
) -> Result<bool, Box<dyn Error>> {
    if watchers
        .game_mode
        .old()
        .is_some_and(|m| m == GameMode::None)
        && (watchers.game_mode.current()? == GameMode::Normal
            || watchers.game_mode.current()? == GameMode::Easy)
    {
        return Ok(true);
    }

    // IL start
    if get_setting("il_mode", settings_defaults)?
        && flag_just_enabled(&watchers.flags, GameFlags::HasStarted)?
    {
        return Ok(true);
    }

    Ok(false)
}
