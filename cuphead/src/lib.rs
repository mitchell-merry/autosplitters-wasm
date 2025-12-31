extern crate helpers;
mod enums;
mod memory;
mod settings;
mod util;

use crate::enums::Mode;
use crate::memory::Memory;
use crate::settings::Settings;
use crate::util::format_seconds;
use asr::future::retry;
use asr::game_engine::unity::mono::Module;
use asr::game_engine::unity::scene_manager::SceneManager;
use asr::settings::Gui;
use asr::timer::{
    pause_game_time, reset, resume_game_time, set_game_time, set_variable, split, start, state,
    TimerState,
};
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::watchers::unity::UnityImage;
use std::error::Error;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 2] = [
    // Windows
    "Cuphead.exe",
    // Mac
    "Cuphead",
];

const SCENE_CUTSCENE_INTRO: &str = "scene_cutscene_intro";
const SCENE_CUTSCENE_KING_DICE_CONTRACT: &str = "scene_cutscene_kingdice";
const SCENE_CUTSCENE_DEVIL: &str = "scene_cutscene_devil";
const SCENE_TITLE_SCREEN: &str = "scene_title";
const SCENE_SCOREBOARD: &str = "scene_win";

const STAR_SKIP_TIME_FIRST: Duration = Duration::from_millis(100);
const STAR_SKIP_TIME_SECOND: Duration = Duration::from_millis(600);
const STAR_SKIP_TIME_THIRD: Duration = Duration::from_millis(1100);

#[derive(Default)]
struct MeasuredState {
    level_updated_lsd: bool,
    lsd_time: f32,
    was_on_scorecard: bool,
    difficulty_ticker_start_time: Option<Instant>,
    difficulty_ticker_end_time: Option<Instant>,
    star_skip_counter: i32,
    star_skip_counter_decimal: i32,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        print_message(&panic_info.to_string());
    }));

    print_message("Hello, World!");

    let mut settings = Settings::register();
    settings.update();

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
        next_tick().await;
    }
}

struct Cuphead<'a> {
    memory: Memory<'a>,
    measured_state: MeasuredState,
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let mut cuphead =
        helpers::try_load::wait_try_load_millis(|| try_load(process), Duration::from_millis(500))
            .await;

    next_tick().await;

    while process.is_open() {
        settings.update();

        next_tick().await;

        cuphead.memory.invalidate();

        if let Err(_err) = tick(&mut cuphead, settings).await {
            // print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn try_load<'a>(process: &'a Process) -> Result<Cuphead<'a>, Box<dyn Error>> {
    print_message("  => loading module");
    let module =
        Module::attach_auto_detect(process).ok_or(SimpleError::from("mono module not found"))?;
    let module = Rc::new(module);
    print_message(&format!(
        "  => module loaded (detected {:?}, {:?}), loading image",
        module.get_version(),
        module.get_pointer_size()
    ));
    next_tick().await;

    let image = module
        .get_default_image(process)
        .ok_or(SimpleError::from("default image not found"))?;
    let unity = UnityImage::new(process, module, image);
    print_message("  => default image loaded, loading scene manager");

    let sm = SceneManager::attach(process)
        .ok_or(SimpleError::from("failed to attach to asr scene manager"))?;
    let sm = Rc::new(sm);
    print_message("  => scene manager loaded, loading pointer paths");

    let memory = Memory::new(unity, sm.clone())?;
    print_message("  => pointer paths loaded");

    Ok(Cuphead {
        memory,
        measured_state: MeasuredState::default(),
    })
}

fn split_log(condition: bool, string: &str) -> bool {
    if condition {
        print_message(&format!("split complete: {string}"));
    }

    condition
}

async fn tick<'a>(
    cuphead: &mut Cuphead<'a>,
    settings: &mut Settings,
) -> Result<(), Box<dyn Error>> {
    let memory = &cuphead.memory;
    let measured_state = &mut cuphead.measured_state;
    let scene = String::from_utf16(memory.scene.current()?.as_slice())?;
    let previous_scene = match memory.scene.old() {
        Some(previous_scene) => String::from_utf16(previous_scene.as_slice())?,
        None => String::new(),
    };

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

    let time = if measured_state.level_updated_lsd {
        measured_state.lsd_time
    } else {
        memory.level_time.current()? + measured_state.lsd_time
    };

    // For users to use directly - key matters
    set_variable("Level Time", &format_seconds(time));

    if state() == TimerState::Running && scene == SCENE_SCOREBOARD {
        monitor_star_skip(memory, measured_state)?;
    }

    let counter = if settings.display_star_skip_counter_as_decimal {
        &format!(
            "{}",
            f32::trunc((measured_state.star_skip_counter_decimal as f32 / 6.0) * 100.0) / 100.0
        )
    } else {
        &format!("{}", measured_state.star_skip_counter)
    };

    set_variable("Star Skip Counter", counter);

    // For run recap component - key matters
    // Future improvement - make these a setting so we save extra performance?
    set_variable("scene name", &scene.to_string());
    set_variable("loading", &format!("{:?}", !memory.done_loading.current()?));
    set_variable(
        "difficulty",
        &format!("{:?}", memory.level_difficulty.current()?),
    );
    set_variable("scoring time", &format!("{}", memory.lsd_time.current()?));
    set_variable("parries", &format!("{}", memory.lsd_parries.current()?));
    set_variable(
        "super meter",
        &format!("{}", memory.lsd_super_meter.current()?),
    );
    set_variable("coins", &format!("{}", memory.lsd_coins.current()?));
    set_variable("hits", &format!("{}", memory.lsd_hits.current()?));
    set_variable(
        "use coins instead of super meter",
        &format!("{}", memory.lsd_use_coins_instead.current()?),
    );

    // For debugging
    #[cfg(debug_assertions)]
    {
        set_variable("insta", &format!("{}", memory.insta.current()?));
        set_variable("in game", &format!("{}", memory.in_game.current()?));
        set_variable("current level", &format!("{:?}", memory.level.current()?));
        set_variable(
            "level ending",
            &format!("{}", memory.level_ending.current()?),
        );
        set_variable("level won", &format!("{}", memory.level_won.current()?));
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
        set_variable(
            "devil bad ending active",
            &format!("{:?}", memory.devil_bad_ending_active.current()?),
        );
        set_variable(
            "difficulty ticker started counting",
            &format!("{:?}", memory.difficulty_ticker_started_counting.current()?),
        );
        set_variable(
            "difficulty ticker finished counting",
            &format!(
                "{:?}",
                memory.difficulty_ticker_finished_counting.current()?
            ),
        );
        set_variable(
            "was on scorecard",
            &format!("{}", measured_state.was_on_scorecard),
        );
        set_variable(
            "lsd time better",
            &format!("{:.2}", measured_state.lsd_time),
        );

        set_variable(
            "level_updated_lsd",
            &format!("{}", measured_state.level_updated_lsd),
        );
    }

    if state() == TimerState::NotRunning {
        measured_state.star_skip_counter = 0;
        measured_state.star_skip_counter_decimal = 0;

        if (scene == SCENE_CUTSCENE_INTRO
        && memory.in_game.current()?
        // just started loading
        && !memory.done_loading.current()?
        && memory.done_loading.old().is_some_and(|l| l))
            || (settings.individual_level_mode
                && memory.level_time.old().is_some_and(|t| t == 0f32)
                && memory.level_time.current()? > 0f32
                && (!memory.level_is_dice.current()? || memory.lsd_time.current()? == 0f32))
        {
            pause_game_time();
            start();
        }
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
        } else if scene == SCENE_CUTSCENE_DEVIL {
            split_log(
                settings.split_devil_deal
                    && memory.devil_bad_ending_active.changed()?
                    && memory.devil_bad_ending_active.current()?,
                "accepted devil deal",
            )
        } else if let Some((from_scene, target_scenes)) = level.split_on_scene_transition_to() {
            // split if the level transitions out to another specific scene (e.g. tutorial)
            split_log(
                level.is_split_enabled(settings)
                    && memory.scene.changed()?
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
                    && memory.level_won.current()?
                    && memory.level_won.old().is_some_and(|w| !w)
                    && (!settings.split_highest_grade
                        || level.get_type().is_highest_grade(
                            memory.level_grade.current()?,
                            memory.level_difficulty.current()?,
                        )),
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
                    && memory.is_loading()?
                    && (!settings.split_highest_grade
                        || level.get_type().is_highest_grade(
                            memory.level_grade.current()?,
                            memory.level_difficulty.current()?,
                        )),
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

fn monitor_star_skip(
    memory: &Memory,
    measured_state: &mut MeasuredState,
) -> Result<(), Box<dyn Error>> {
    let difficulty_ticker_started_counting = memory.difficulty_ticker_started_counting.current()?;
    let difficulty_ticker_finished_counting =
        memory.difficulty_ticker_finished_counting.current()?;
    if !difficulty_ticker_started_counting {
        measured_state.difficulty_ticker_start_time = None;
        measured_state.difficulty_ticker_end_time = None;
        return Ok(());
    }

    let start = match measured_state.difficulty_ticker_start_time {
        Some(time) => time,
        None => Instant::now(),
    };
    measured_state.difficulty_ticker_start_time = Some(start);
    if !difficulty_ticker_finished_counting || measured_state.difficulty_ticker_end_time.is_some() {
        return Ok(());
    }

    let end = Instant::now();
    measured_state.difficulty_ticker_end_time = Some(end);

    let diff: Duration = end.duration_since(start);

    match memory.level_difficulty.current()? {
        Mode::Easy => {
            if diff < STAR_SKIP_TIME_FIRST {
                measured_state.star_skip_counter += 1;
                measured_state.star_skip_counter_decimal += 6;
            }
        }

        Mode::Normal => {
            if diff < STAR_SKIP_TIME_FIRST {
                measured_state.star_skip_counter += 2;
                measured_state.star_skip_counter_decimal += 6;
            } else if diff < STAR_SKIP_TIME_SECOND {
                measured_state.star_skip_counter += 1;
                measured_state.star_skip_counter_decimal += 3;
            }
        }

        Mode::Hard => {
            if diff < STAR_SKIP_TIME_FIRST {
                measured_state.star_skip_counter += 3;
                measured_state.star_skip_counter_decimal += 6;
            } else if diff < STAR_SKIP_TIME_SECOND {
                measured_state.star_skip_counter += 2;
                measured_state.star_skip_counter_decimal += 4;
            } else if diff < STAR_SKIP_TIME_THIRD {
                measured_state.star_skip_counter += 1;
                measured_state.star_skip_counter_decimal += 2;
            }
        }

        _ => {}
    }

    Ok(())
}
