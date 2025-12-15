use helpers::watchers::unity::UnityImage;
use helpers::watchers::Watcher;

pub struct Memory<'a> {
    pub combat_time: Watcher<'a, f32>,
}

impl<'a> Memory<'a> {
    pub fn new(unity: UnityImage<'a>) -> Memory<'a> {
        Memory {
            combat_time: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timer"],
            )),
        }
    }

    pub fn invalidate(&mut self) {
        self.combat_time.invalidate();
    }
}
