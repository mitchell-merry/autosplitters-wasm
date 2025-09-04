use asr::game_engine::unity::mono::Class;

pub struct Classes {
    pub scene_loader: Class,
}

#[derive(Class)]
pub struct SceneLoader {
    #[rename = "doneLoadingSceneAsync"]
    pub is_loading: bool,
}
