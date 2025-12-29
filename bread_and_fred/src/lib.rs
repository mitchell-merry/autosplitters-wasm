extern crate helpers;
mod memory;
mod settings;

use crate::memory::Memory;
use crate::settings::Settings;
use asr::future::retry;
use asr::game_engine::unity::mono::Module;
use asr::game_engine::unity::scene_manager::SceneManager;
use asr::settings::Gui;
use asr::timer::{pause_game_time, resume_game_time, set_game_time, set_variable, TimerState};
use asr::{future::next_tick, print_message, timer, Process};
use core::time::Duration;
use helpers::error::SimpleError;
use helpers::watchers::unity::UnityImage;
use std::error::Error;
use std::rc::Rc;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 1] = [
    // Windows
    // "Cuphead.exe",
    // Mac
    "Bread_Fred",
];

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

struct Game<'a> {
    memory: Memory<'a>,
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let mut game =
        helpers::try_load::wait_try_load_millis(|| try_load(process), Duration::from_millis(500))
            .await;

    next_tick().await;

    while process.is_open() {
        settings.update();

        next_tick().await;

        game.memory.invalidate();

        if let Err(_err) = tick(&mut game, settings).await {
            // print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn try_load<'a>(process: &'a Process) -> Result<Game<'a>, Box<dyn Error>> {
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

    // print_message(&format!(
    //     "off {:X?}",
    //     image
    //         .get_class(process, &module, "Timer")
    //         .unwrap()
    //         .get_field_offset(process, &module, "_currentRunTimer")
    //         .unwrap()
    // ));

    let unity = UnityImage::new(process, module, image);
    print_message("  => default image loaded, loading scene manager");

    let sm = SceneManager::attach(process)
        .ok_or(SimpleError::from("failed to attach to asr scene manager"))?;
    let sm = Rc::new(sm);
    print_message("  => scene manager loaded, loading pointer paths");

    let memory = Memory::new(unity, sm.clone())?;
    print_message("  => pointer paths loaded");

    Ok(Game { memory })
}

fn split_log(condition: bool, string: &str) -> bool {
    if condition {
        print_message(&format!("split complete: {string}"));
    }

    condition
}

async fn tick<'a>(game: &mut Game<'a>, _settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let memory = &game.memory;

    // set_variable("timer instance", &format!("{:?}", memory.timer.current()));
    let elapsed = memory.time.current()? as f32 / 10_000_000.0;
    set_variable("time", &format!("{:?}", elapsed));
    set_variable(
        "started timestamp",
        &format!("{:?}", memory.started_timestamp.current()?),
    );
    set_variable(
        "current timestamp",
        &format!(
            "{:?}",
            // Instant::now().duration_since(Instant::from(memory.started_timestamp.current()?))
            std::time::SystemTime::now()
        ),
    );
    set_variable("running", &format!("{:?}", memory.running.current()?));
    // set_variable(
    //     "game manager instance",
    //     &format!("{:?}", memory.game_manager.current()),
    // );

    if timer::state() == TimerState::NotRunning
        && memory.running.changed()?
        && memory.running.current()?
    {
        timer::start();
    }

    if timer::state() == TimerState::Running {
        if memory.running.current()? {
            resume_game_time();
        } else {
            pause_game_time();
        }

        if memory.time.changed()? {
            let time = asr::time::Duration::nanoseconds(memory.time.current()? as i64 * 100);
            print_message(&format!(
                "setting game time to {}",
                time.whole_milliseconds()
            ));
            pause_game_time();
            set_game_time(time);
        }
    }

    Ok(())
}
