use asr::game_engine::unity::scene_manager::SceneManager;
use helpers::watchers::unity::UnityImage;
use std::error::Error;
use std::marker::PhantomData;
use std::rc::Rc;

pub struct Memory<'a> {
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        Ok(Memory {
            _phantom: PhantomData,
        })
    }

    pub fn invalidate(&mut self) {}
}
