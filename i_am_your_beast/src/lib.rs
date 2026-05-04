extern crate helpers_iayb;
mod enums;
mod memory;
mod settings;

use crate::enums::{LevelState, SceneTransitionState};
use crate::memory::Memory;
use crate::settings::Settings;
use asr::future::retry;
use asr::game_engine::unity::mono::Module;
use asr::game_engine::unity::scene_manager::SceneManager;
use asr::settings::Gui;
use asr::timer::{
    pause_game_time, reset, resume_game_time, set_game_time, set_variable, split, state, TimerState,
};
use asr::{future::next_tick, print_message, timer, Process};
use helpers_iayb::error::SimpleError;
use helpers_iayb::watchers::unity::UnityImage;
use std::error::Error;
use std::rc::Rc;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 1] = [
    // Windows
    "I Am Your Beast.exe",
];

const SCENE_LEVEL_SELECT: &str = "LevelSelect";
const SCENE_CUTSCENE: &str = "Cutscene";
const SCENE_TUTORIAL_1: &str = "#01a_Special_Tutorial";
const SCENE_TUTORIAL_2: &str = "#01c_Special_Tutorial";
const SCENE_WALKOUT: &str = "#2_Corridor_WabbitSeason";
const SCENE_MERCY: &str = "#25_Special_Blinded";

const SCENE_CHALLENGE_BABA_YAGA: &str = "#1_Challenge_FirstSteps";
const SCENE_SG_PITCH_BLACK: &str = "#1_PLP_PitchBlack";
const SCENE_CS_LIMBO: &str = "##_CS_Limbo";

const RTA_SCENE_STARTS: &[&str] = &[
    SCENE_TUTORIAL_1,
    SCENE_CHALLENGE_BABA_YAGA,
    SCENE_SG_PITCH_BLACK,
    SCENE_CS_LIMBO,
];

const IGT_SCENE_STARTS: &[&str] = &[
    SCENE_WALKOUT,
    SCENE_CHALLENGE_BABA_YAGA,
    SCENE_SG_PITCH_BLACK,
    SCENE_CS_LIMBO,
];

#[derive(Default)]
struct MeasuredState {
    total_igt: f32,
    level_started: bool,
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

struct IAmYourBeast<'a> {
    memory: Memory<'a>,
    measured_state: MeasuredState,
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let mut iamyourbeast = helpers_iayb::try_load::wait_try_load_millis::<IAmYourBeast, _, _>(
        async || {
            print_message("  => loading module");
            let module = Module::attach_auto_detect(process)
                .ok_or(SimpleError::from("mono module not found"))?;
            let module = Rc::new(module);
            print_message(&format!(
                "  => module loaded (detected {:?}, {:?}), loading image",
                module.get_version(),
                module.get_pointer_size()
            ));

            let image = module
                .get_default_image(process)
                .ok_or(SimpleError::from("default image not found"))?;
            let unity = UnityImage::new(process, module, image);
            print_message("  => image loaded, loading scene manager");

            let sm = SceneManager::attach(process)
                .ok_or(SimpleError::from("failed to attach to asr scene manager"))?;
            let sm = Rc::new(sm);
            print_message("  => scene manager loaded, loading pointer paths");

            let memory = Memory::new(unity, sm.clone())?;
            print_message("  => pointer paths loaded");

            Ok(IAmYourBeast {
                memory,
                measured_state: MeasuredState {
                    total_igt: 3600f32,
                    level_started: Default::default(),
                },
            })
        },
        std::time::Duration::from_millis(500),
    )
    .await;

    while process.is_open() {
        settings.update();

        next_tick().await;

        iamyourbeast.memory.invalidate();

        if let Err(err) = tick(process, &mut iamyourbeast, settings).await {
            print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn tick<'a>(
    _process: &'a Process,
    iamyourbeast: &mut IAmYourBeast<'a>,
    settings: &mut Settings,
) -> Result<(), Box<dyn Error>> {
    let memory = &iamyourbeast.memory;
    let old_scene = memory.scene.old().unwrap_or("".to_string());
    let scene = memory.scene.current()?;
    let measured_state = &mut iamyourbeast.measured_state;
    let current_ui_time = (memory.ui_level_complete_time.current()? * 100f32).round() / 100f32;

    set_variable("combat time", &format!("{}", memory.combat_time.current()?));
    set_variable(
        "ui_level_complete_time",
        &format!("{:?}", memory.ui_level_complete_time.current()?),
    );
    set_variable(
        "ui_level_complete_time rounded",
        &format!("{current_ui_time}"),
    );
    set_variable("level", &format!("{:?}", memory.level.current()?));
    set_variable(
        "level_state",
        &format!("{:?}", memory.level_state.current()?),
    );
    set_variable("tracking", &format!("{:?}", memory.tracking.current()?));
    set_variable(
        "cutscene_id",
        &format!("{:?}", memory.cutscene_id.current()?),
    );
    set_variable("scene", &scene);
    set_variable(
        "scene_transition_state",
        &format!("{:?}", memory.scene_transition_state.current()?),
    );
    set_variable(
        "combat_time",
        &format!("{:?}", memory.combat_time.current()?),
    );
    set_variable(
        "regained_combat_time",
        &format!("{:?}", memory.regained_combat_time.current()?),
    );
    set_variable(
        "timer_started",
        &format!("{:?}", memory.timer_started.current()?),
    );
    set_variable(
        "level started",
        &format!("{:?}", measured_state.level_started),
    );
    set_variable(
        "credits active",
        &format!("{:?}", memory.credits_active.current()?),
    );
    set_variable(
        "credits index",
        &format!("{:?}", memory.credits_index.current()?),
    );

    if state() == TimerState::NotRunning {
        measured_state.total_igt = 3600f32;

        let should_start = if settings.use_in_game_time {
            let igt_just_started = memory.timer_started.changed_from_to(false, true)?;

            igt_just_started
                && (settings.individual_level_mode || IGT_SCENE_STARTS.contains(&scene.as_str()))
        } else {
            let just_loaded_into_level = memory
                .scene_transition_state
                .is(SceneTransitionState::TransitioningIn)?
                && memory.tracking.changed_from_to(false, true)?;

            just_loaded_into_level
                && (settings.individual_level_mode || RTA_SCENE_STARTS.contains(&scene.as_str()))
        };

        if should_start {
            timer::start();
        }
    }

    if state() == TimerState::Running {
        if settings.use_in_game_time {
            pause_game_time();

            /*
             * Full game IGT logic
             * 1. Complete level - on ui level complete, add IGT time and an hour
             * 2. Failed level - on Failed, add combat time
             * 3. Exit to level - add combat time
             * 4. Restart level - add combat time
             */

            let should_split = memory.ui_level_complete_time.changed_from(0f32)?;
            if should_split {
                measured_state.total_igt += current_ui_time;
                measured_state.level_started = false;
            }

            if memory.level_state.changed_to(LevelState::Failed)? {
                measured_state.total_igt += memory.combat_time.current()?;
                measured_state.level_started = false;
            }

            if memory
                .scene_transition_state
                .changed_from(SceneTransitionState::TransitioningIn)?
            {
                if measured_state.level_started {
                    measured_state.total_igt += memory.combat_time.current()?;
                }

                measured_state.level_started = false;
            }

            if memory.combat_time.changed_from(0f32)? {
                measured_state.level_started = true;
            }

            let time = if measured_state.level_started && memory.ui_level_complete_time.is(0f32)? {
                measured_state.total_igt + memory.combat_time.current()?
                    - memory.regained_combat_time.current()?
            } else {
                measured_state.total_igt
            };

            set_game_time(asr::time::Duration::seconds_f32(time));

            if should_split {
                split();
                measured_state.total_igt += 3600f32;
            }
        } else {
            let scene_transitioning = !memory
                .scene_transition_state
                .is(SceneTransitionState::TransitioningIn)?;

            let should_pause = match scene.as_str() {
                SCENE_TUTORIAL_1 | SCENE_TUTORIAL_2 => scene_transitioning,
                SCENE_LEVEL_SELECT => true,
                _ => {
                    scene_transitioning
                        || memory.level_state.is(LevelState::Intro)?
                        || memory.level_state.is(LevelState::Completed)?
                }
            };

            if should_pause {
                pause_game_time();
            } else {
                resume_game_time();
            }

            let should_split = match old_scene.as_str() {
                SCENE_CUTSCENE => {
                    memory.cutscene_id.current()? == 22 && scene == SCENE_LEVEL_SELECT
                }
                SCENE_TUTORIAL_2 => scene == SCENE_LEVEL_SELECT,
                _ => memory
                    .level_state
                    .changed_from_to(LevelState::Active, LevelState::Completed)?,
            };

            if should_split {
                split();
            }
        }

        // credits, for 100% runs
        if memory.credits_active.changed_from_to(true, false)? && memory.credits_index.is(49)? {
            split();
        }

        if settings.individual_level_mode {
            let should_reset = match scene.as_str() {
                SCENE_TUTORIAL_2 | SCENE_MERCY => false,
                _ => memory
                    .scene_transition_state
                    .changed_to(SceneTransitionState::TransitioningOut)?,
            };

            if should_reset {
                reset();
            }
        }
    }

    Ok(())
}
