extern crate helpers;
mod settings;

use crate::settings::Settings;
use asr::future::retry;
use asr::game_engine::unity::scene_manager::{Scene, SceneManager, Transform};
use asr::settings::Gui;
use asr::string::ArrayCString;
use asr::{future::next_tick, print_message, Address, Process};
use core::time::Duration;
use helpers::error::SimpleError;
use std::error::Error;
use std::rc::Rc;
use asr::game_engine::unity::mono;
use asr::game_engine::unity::mono::Module;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 6] = [
    // Windows
    "Bread&Fred.exe",
    "platformer-playground.exe",
    // Mac
    "My project",
    "My project 6000",
    "My project 6000.exe",
    "Bread_Fred",
];

trait UnityModule {
    fn get_class_name(&self, process: &Process, class: Address) -> Result<String, Box<dyn Error>>;
    fn get_class(&self, process: &Process, class_name: &str) -> Result<Address, Box<dyn Error>>;
}

struct MonoModule {
    pub module: mono::Module,
}

impl MonoModule {
    pub fn attach(process: &Process) -> Result<Self, Box<dyn Error>> {
        let module = Module::attach_auto_detect(process).ok_or(SimpleError::from("cant auto attach module"))?;
        Ok(MonoModule { module })
    }
}

impl UnityModule for MonoModule {
    fn get_class_name(&self, process: &Process, class: Address) -> Result<String, Box<dyn Error>> {
        safe_cstr_to_str(mono::Class::get_from_component(process, &self.module, class)
            .and_then(|class| class.get_name::<128>(process, &self.module)))
    }

    fn get_class(&self, process: &Process, class_name: &str) -> Result<Address, Box<dyn Error>> {
        let image = self.module.get_default_image(process).ok_or(SimpleError::from("cant get default image"))?;

        Ok(image.get_class(process, &self.module, class_name).ok_or(SimpleError::from("cant get class"))?.class)
    }
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
                    print_message(&format!("error occurring on_attach: {}", err));
                } else {
                    print_message("detached from process");
                }
            })
            .await;
        next_tick().await;
    }
}

struct Game {
    pub scene_manager: Rc<SceneManager>,
    pub module: Box<dyn UnityModule>,
}

pub fn safe_cstr_to_str<const N: usize>(
    cstr: Result<ArrayCString<N>, asr::Error>,
) -> Result<String, Box<dyn Error>> {
    match cstr {
        Err(_) => Err(SimpleError::from("failed to read string").into()),
        Ok(cstr) => cstr
            .validate_utf8()
            .map_err(|_| SimpleError::from("failed to unmarshal string").into())
            .map(|s| s.to_owned()),
    }
}

fn log_transform(process: &Process, game: &Game, transform: Transform, indent: &str) {
    let name = safe_cstr_to_str(transform.get_name::<128>(process, &game.scene_manager));
    print_message(&format!("{indent}NAME: {name:?}"));

    let game_object = transform
        .get_game_object(process, &game.scene_manager);

    if let Ok(game_object) = game_object {
        let active_self = game_object.is_active_self(process, &game.scene_manager);
        print_message(&format!("{indent}ACTIVE SELF: {active_self:?}"));

        let active_in_hierarchy = game_object.is_active_in_hierarchy(process, &game.scene_manager);
        print_message(&format!(
            "{indent}ACTIVE IN HIERARCHY: {active_in_hierarchy:?}"
        ));

        if let Ok(classes) = game_object.classes(process, &game.scene_manager) {
            print_message(&format!("{indent}CLASSES:"));
            classes.enumerate().for_each(|(i, class)| {
                print_message(&format!("{indent}  CLASS: {class:?}"));
                let name = game.module.get_class_name(process, class);

                if let Ok(name) = name {
                    print_message(&format!("{indent}    NAME (MONO): {name:?}"));
                }
            })
        }
    }

    if let Ok(children) = transform.children(process, &game.scene_manager) {
        print_message(&format!("{indent}CHILDREN:"));
        children.enumerate().for_each(|(i, y)| {
            print_message(&format!("{indent}  CHILD {i}: {y:?}"));

            log_transform(process, game, y, &format!("{indent}    "));
        });
    }
}

fn log_scene(process: &Process, game: &Game, s: Scene, indent: &str) {
    let path = safe_cstr_to_str(s.path::<128>(process, &game.scene_manager));
    print_message(&format!("{indent}PATH: {path:?}"));
    let index = s.index(process, &game.scene_manager);
    print_message(&format!("{indent}BUILD INDEX: {index:?}"));
    print_message(&format!("{indent}ROOT GAME OBJECTS:"));

    s.root_game_objects(process, &game.scene_manager)
        .enumerate()
        .for_each(|(i, x)| {
            print_message(&format!("{indent}  TRANSFORM {i}: {x:?}:"));
            log_transform(process, game, x, &format!("{indent}    "));
        });
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let mut game =
        helpers::try_load::wait_try_load_millis(|| try_load(process), Duration::from_millis(500))
            .await;

    next_tick().await;

    let s = game
        .scene_manager
        .get_current_scene(process)
        .map_err(|_| SimpleError::from("failed to get current scene"))?;
    print_message(&format!("ACTIVE SCENE: {s:?}"));
    log_scene(process, &game, s, "  ");

    print_message(&format!("RestartScript {:?}", game.module.get_class(process, "RestartScript")));

    while process.is_open() {
        settings.update();

        next_tick().await;

        if let Err(_err) = tick(&mut game, settings).await {
            // print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn try_load<'a>(process: &'a Process) -> Result<Game, Box<dyn Error>> {
    print_message("  => loading scene manager");

    let sm = SceneManager::attach(process)
        .ok_or(SimpleError::from("failed to attach to asr scene manager"))?;
    let sm = Rc::new(sm);
    print_message("  => scene manager loaded, loading unity module");

    let module = MonoModule::attach(process)?;

    Ok(Game { scene_manager: sm, module: Box::new(module) })
}

fn split_log(condition: bool, string: &str) -> bool {
    if condition {
        print_message(&format!("split complete: {string}"));
    }

    condition
}

async fn tick<'a>(game: &mut Game, _settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    Ok(())
}
