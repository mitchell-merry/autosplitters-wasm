extern crate helpers;
mod memory;
mod settings;

use crate::memory::Memory;
use crate::settings::Settings;
use asr::future::retry;
use asr::game_engine::unity::mono::Module;
use asr::game_engine::unity::scene_manager::SceneManager;
use asr::settings::Gui;
use asr::timer::{pause_game_time, resume_game_time, set_variable, start, state, TimerState};
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::watchers::unity::UnityImage;
use std::error::Error;
use std::rc::Rc;
use std::time::Duration;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 2] = [
    // Windows
    "BALDI.exe",
    // Mac
    "Baldi",
];

#[derive(Default)]
struct MeasuredState {}

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

struct Baldi<'a> {
    memory: Memory<'a>,
    measured_state: MeasuredState,
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let mut baldi =
        helpers::try_load::wait_try_load_millis(|| try_load(process), Duration::from_millis(500))
            .await;

    next_tick().await;

    while process.is_open() {
        settings.update();

        next_tick().await;

        baldi.memory.invalidate();

        if let Err(err) = tick(&mut baldi, settings).await {
            print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn try_load<'a>(process: &'a Process) -> Result<Baldi<'a>, Box<dyn Error>> {
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

    Ok(Baldi {
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

async fn tick<'a>(baldi: &mut Baldi<'a>, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let memory = &baldi.memory;
    let measured_state = &mut baldi.measured_state;

    set_variable("velocity", &format!("{:?}", &memory.velocity.current()));
    set_variable("scene", &format!("{:?}", &memory.scene.current()));
    set_variable(
        "ready_to_start",
        &format!("{:?}", &memory.ready_to_start.current()),
    );
    set_variable(
        "transition_active",
        &format!("{:?}", &memory.transition_active.current()),
    );
    set_variable("x", &format!("{:X?}", &memory.x.current()));
    set_variable("xx", &format!("{:X?}", &memory.xx.current()));

    if state() == TimerState::NotRunning {
        if memory.velocity.changed_from(0f32)? {
            start();
        }
    }

    if state() == TimerState::Running {
        let is_loading =
            // from main menu into game (never usually in runs anyways)
            // TODO we really should just be checking on if the scene manager is loading
            //  for the first
            !memory.ready_to_start.current()?
            // if we're transitioning
            || (settings.pause_on_transition && memory.transition_active.current()?);

        if is_loading {
            pause_game_time();
        } else {
            resume_game_time();
        }
    }

    Ok(())
}
