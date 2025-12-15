use asr::Process;
use helpers::watchers::pointer_path::PointerPath;
use helpers::watchers::Watcher;
use idtech::IdTech;
use std::error::Error;

pub struct IdVec3<'a> {
    pub x: Watcher<'a, f32>,
    pub y: Watcher<'a, f32>,
    pub z: Watcher<'a, f32>,
}

impl<'a> IdVec3<'a> {
    pub fn init(
        idtech: &IdTech<'a>,
        path: PointerPath<'a, Process>,
    ) -> Result<Self, Box<dyn Error>> {
        let c = idtech.get_class("Engine", "idVec3")?;
        Ok(IdVec3 {
            x: path.child([c.get_offset("x")?]).into(),
            y: path.child([c.get_offset("y")?]).into(),
            z: path.child([c.get_offset("z")?]).into(),
        })
    }

    pub fn invalidate(&mut self) {
        self.x.invalidate();
        self.y.invalidate();
        self.z.invalidate();
    }
}
