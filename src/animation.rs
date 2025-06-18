use crate::config::ANIM_DURATION_MS;
use crate::ui::AssetPaths;
use glib::source::timeout_add_local;
use glib::ControlFlow;
use gtk4::Picture;
use std::cell::Cell;
use std::time::Duration;

pub struct Animator {
    next_left: Cell<bool>,
    assets: AssetPaths,
}

impl Animator {
    pub fn new(assets: AssetPaths) -> Self {
        Self {
            next_left: Cell::new(true),
            assets,
        }
    }

    pub fn animate(&self, picture: &Picture) {
        let use_left = self.next_left.get();
        self.next_left.set(!use_left);

        let hit_path = if use_left {
            &self.assets.hit_left
        } else {
            &self.assets.hit_right
        };

        let idle_path = &self.assets.idle;
        let delay = Duration::from_millis(ANIM_DURATION_MS as u64);

        // Show hit animation
        picture.set_filename(Some(hit_path));

        // Schedule return to idle
        let picture_clone = picture.clone();
        let idle_path_clone = idle_path.clone();

        timeout_add_local(delay, move || {
            picture_clone.set_filename(Some(&idle_path_clone));
            ControlFlow::Break
        });
    }
}
