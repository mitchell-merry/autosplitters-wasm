extern crate helpers;
mod memory;
mod settings;

use crate::memory::Memory;
use crate::settings::Settings;
use asr::future::retry;
use asr::game_engine::unity::mono::Module;
use asr::game_engine::unity::scene_manager::SceneManager;
use asr::settings::Gui;
use asr::timer::set_variable;
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::watchers::unity::UnityImage;
use std::error::Error;
use std::rc::Rc;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 1] = [
    // Windows
    "I Am Your Beast.exe",
];

#[derive(Default)]
struct MeasuredState {}

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
    let mut iamyourbeast = helpers::try_load::wait_try_load_millis::<IAmYourBeast, _, _>(
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
                measured_state: MeasuredState::default(),
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
    set_variable(
        "combat time",
        &format!("{}", iamyourbeast.memory.combat_time.current()?),
    );
    set_variable(
        "ui_level_complete_time",
        &format!("{:?}", iamyourbeast.memory.ui_level_complete_time.current()),
    );
    set_variable(
        "level",
        &format!("{:?}", iamyourbeast.memory.level.current()),
    );
    set_variable(
        "level_state",
        &format!("{:?}", iamyourbeast.memory.level_state.current()),
    );
    set_variable(
        "tracking",
        &format!("{:?}", iamyourbeast.memory.tracking.current()),
    );
    set_variable(
        "cutscene_id",
        &format!("{:?}", iamyourbeast.memory.cutscene_id.current()),
    );
    set_variable(
        "transition_scene",
        &format!(
            "{:X?}",
            String::from_utf16(iamyourbeast.memory.transition_scene.current()?.as_slice())
        ),
    );
    set_variable(
        "scene_transition_state",
        &format!("{:?}", iamyourbeast.memory.scene_transition_state.current()),
    );
    set_variable(
        "combat_time",
        &format!("{:?}", iamyourbeast.memory.combat_time.current()),
    );
    set_variable(
        "regained_combat_time",
        &format!("{:?}", iamyourbeast.memory.regained_combat_time.current()),
    );
    set_variable(
        "ui_level_complete_time",
        &format!("{:?}", iamyourbeast.memory.ui_level_complete_time.current()),
    );
    set_variable(
        "timer_started",
        &format!("{:?}", iamyourbeast.memory.timer_started.current()),
    );

    Ok(())
}
