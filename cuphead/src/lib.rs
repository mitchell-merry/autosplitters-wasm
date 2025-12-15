extern crate helpers;
mod enums;
mod memory;
mod settings;
mod util;

use crate::memory::Memory;
use crate::settings::Settings;
use crate::util::format_seconds;
use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module};
use asr::settings::Gui;
use asr::timer::{
    pause_game_time, reset, resume_game_time, set_game_time, set_variable, split, start, state,
    TimerState,
};
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::watchers::unity::UnityImage;
use std::error::Error;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 2] = [
    // Windows
    "Cuphead.exe",
    // Mac
    "Cuphead",
];

const SCENE_CUTSCENE_INTRO: &str = "scene_cutscene_intro";
const SCENE_CUTSCENE_KING_DICE_CONTRACT: &str = "scene_cutscene_kingdice";
const SCENE_TITLE_SCREEN: &str = "scene_title";

#[derive(Default)]
struct MeasuredState {
    level_updated_lsd: bool,
    lsd_time: f32,
    was_on_scorecard: bool,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        print_message(&panic_info.to_string());
    }));

    print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = retry(|| PROCESS_NAMES.iter().find_map(|name| Process::attach(name))).await;

        process
            .until_closes(async {
                let res = on_attach(&process, &mut settings).await;
                if let Err(err) = res {
                    print_message(&format!("error occuring on_attach: {}", err));
                } else {
                    print_message("detached from process");
                }
            })
            .await;
    }
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let (module, image) = helpers::try_load::wait_try_load_millis::<(Module, Image), _, _>(
        async || {
            print_message("  => loading module");
            let module = Module::attach_auto_detect(process)
                .ok_or(SimpleError::from("mono module not found"))?;
            print_message(&format!(
                "  => module loaded (detected {:?}, {:?}), loading image",
                module.version, module.pointer_size
            ));
            let image = module
                .get_default_image(process)
                .ok_or(SimpleError::from("default image not found"))?;
            // let scene_manager = SceneManager::attach(process)
            //     .ok_or(SimpleError::from("scene manager not found"))?;

            Ok((module, image))
        },
        std::time::Duration::from_millis(500),
    )
    .await;

    let unity = UnityImage::new(process, &module, &image);
    let mut memory = Memory::new(unity);
    let mut measured_state = MeasuredState::default();

    while process.is_open() {
        settings.update();

        next_tick().await;

        memory.invalidate();

        if let Err(err) = tick(process, &memory, &mut measured_state, settings).await {
            // print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

fn split_log(condition: bool, string: &str) -> bool {
    if condition {
        print_message(&format!("split complete: {string}"));
    }

    condition
}

async fn tick<'a>(
    process: &'a Process,
    memory: &Memory<'a>,
    measured_state: &mut MeasuredState,
    settings: &mut Settings,
) -> Result<(), Box<dyn Error>> {
    // Intended for users:

    set_variable(
        "done loading scene async",
        &format!("{}", memory.done_loading.current()?),
    );
    let scene = String::from_utf16(memory.scene.current()?.as_slice())?;
    set_variable("scene name", &format!("{}", scene));
    let previous_scene = match memory.scene.old() {
        Some(previous_scene) => String::from_utf16(previous_scene.as_slice())?,
        None => String::new(),
    };

    set_variable("in game", &format!("{}", memory.in_game.current()?));
    set_variable("current level", &format!("{:?}", memory.level.current()?));
    set_variable("level won", &format!("{}", memory.level_won.current()?));
    set_variable(
        "level ending",
        &format!("{}", memory.level_ending.current()?),
    );
    set_variable(
        "level time (raw)",
        &format!(
            "{}",
            f32::trunc(memory.level_time.current()? * 100.0) / 100.0
        ),
    );
    set_variable(
        "level time (ind)",
        &format!("{:.2}", memory.level_time.current()?),
    );
    set_variable("lsd time (raw)", &format!("{}", memory.lsd_time.current()?));
    set_variable(
        "kd spaces moved",
        &format!("{}", memory.kd_spaces_moved.current()?),
    );
    set_variable(
        "is dice palace",
        &format!("{}", memory.level_is_dice.current()?),
    );
    set_variable(
        "is dice palace main",
        &format!("{}", memory.level_is_dice_main.current()?),
    );

    if memory.lsd_time.changed()? && memory.lsd_time.current()? != 0f32 {
        measured_state.lsd_time = memory.lsd_time.current()?;
        measured_state.level_updated_lsd = true
    }

    let level_is_resetting = if memory.level_is_dice.current()? {
        memory.kd_spaces_moved.current()? == 0
            && memory.is_loading()?
            && memory.done_loading.old().is_some_and(|l| !l)
    } else {
        memory.level_time.old().is_some_and(|t| t > 0f32) && memory.level_time.current()? == 0f32
    };

    if level_is_resetting {
        measured_state.lsd_time = 0f32;
        measured_state.level_updated_lsd = false;
    }

    if memory.level.changed()? {
        measured_state.level_updated_lsd = false;
    }

    if !measured_state.was_on_scorecard {
        measured_state.was_on_scorecard = previous_scene == "scene_win" && scene != "scene_win";
    }
    set_variable(
        "was on scorecard",
        &format!("{}", measured_state.was_on_scorecard),
    );

    let time = if measured_state.level_updated_lsd {
        measured_state.lsd_time
    } else {
        memory.level_time.current()? + measured_state.lsd_time
    };

    set_variable("Level Time", &format_seconds(time));
    set_variable(
        "lsd time better",
        &format!("{:.2}", measured_state.lsd_time),
    );

    set_variable(
        "level_updated_lsd",
        &format!("{}", measured_state.level_updated_lsd),
    );

    if state() == TimerState::NotRunning
        && ((scene == SCENE_CUTSCENE_INTRO
        && memory.in_game.current()?
        // just started loading
        && !memory.done_loading.current()?
        && memory.done_loading.old().is_some_and(|l| l))
            || (settings.individual_level_mode
                && memory.level_time.old().is_some_and(|t| t == 0f32)
                && memory.level_time.current()? > 0f32
                && (!memory.level_is_dice.current()? || memory.lsd_time.current()? == 0f32)))
    {
        pause_game_time();
        start();
    }

    if state() == TimerState::Running {
        if memory.done_loading.changed()? {
            print_message("  => done loading changed");
        }

        if settings.individual_level_mode {
            pause_game_time();
            set_game_time(asr::time::Duration::seconds_f32(time));
        } else if memory.is_loading()? {
            pause_game_time();
        } else {
            resume_game_time();
        }

        let level = memory.level.current()?;
        let should_split = if scene == SCENE_CUTSCENE_KING_DICE_CONTRACT {
            // we do this first because the level is whatever the previous level was (usually Train)
            // so none of the level-specific logic makes sense
            split_log(
                settings.split_kd_contract_cutscene
                    && previous_scene != SCENE_CUTSCENE_KING_DICE_CONTRACT,
                "king dice contract",
            )
        } else if let Some((from_scene, target_scenes)) = level.split_on_scene_transition_to() {
            // split if the level transitions out to another specific scene (e.g. tutorial)
            split_log(
                memory.scene.changed()?
                    && previous_scene == from_scene
                    && target_scenes.contains(scene.as_str()),
                &format!("scene change ({} -> {})", from_scene, scene.as_str()),
            )
        } else if settings
            .split_level_complete
            .should_split_on_knockout(level)
            || settings.individual_level_mode
        {
            // split on knockout
            split_log(
                level.is_split_enabled(settings)
                    && memory.level_won.old().is_some_and(|w| !w)
                    && memory.level_won.current()?,
                &format!("knockout ({:?})", level),
            )
        } else {
            // split after scoreboard
            // split when we start loading, this gives cleaner splits (segment timer is at 0.00 in
            //   the loading screen)
            split_log(
                level.is_split_enabled(settings)
                    && measured_state.was_on_scorecard
                    && memory.done_loading.changed()?
                    && memory.is_loading()?,
                &format!("after scoreboard ({:?})", level),
            )
        };

        if should_split {
            split();
        }

        if scene == SCENE_TITLE_SCREEN && settings.auto_reset
            || settings.individual_level_mode && level_is_resetting
        {
            reset();
        }
    }

    if measured_state.was_on_scorecard && memory.done_loading.changed()? && memory.is_loading()? {
        measured_state.was_on_scorecard = false;
    }

    Ok(())
}
