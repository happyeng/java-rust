use crate::verifier::annoucement::Announcement;
use biodivine_lib_bdd::Bdd;

#[derive(Clone)]
pub struct Ctx {
    device_name: String,
    announcement: Announcement,
}

impl Ctx {
    pub fn new(device_name: String, predicate: Bdd, count: i32) -> Self {
        let announcement = Announcement::new(predicate, count);
        Ctx {
            device_name,
            announcement,
        }
    }

    pub fn set_device_name(&mut self, new_device_name: String) {
        self.device_name = new_device_name;
    }

    pub fn get_device_name(&self) -> String {
        self.device_name.to_string()
    }

    pub fn set_announcement(&mut self, new_announcement: Announcement) {
        self.announcement = new_announcement;
    }

    pub fn get_announcement(&self) -> Announcement {
        self.announcement.clone()
    }
}
