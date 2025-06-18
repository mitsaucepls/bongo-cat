use std::env;
use std::path::PathBuf;

pub const ANIM_DURATION_MS: u32 = 150;
pub const APP_ID: &str = "com.example.BongoCat";
pub const DB_FILENAME: &str = "sqlite.db";

pub const IDLE_ASSET: &str = "idle.png";
pub const HIT_LEFT_ASSET: &str = "hit_left.png";
pub const HIT_RIGHT_ASSET: &str = "hit_right.png";

pub const WINDOW_MARGIN_BOTTOM: i32 = 93;
pub const WINDOW_MARGIN_RIGHT: i32 = 7;

pub fn asset_dir() -> PathBuf {
    if let Some(val) = env::var_os("BONGO_ASSETS") {
        return PathBuf::from(val);
    }

    dirs::config_dir()
        .expect("could not find a config directory")
        .join("bongo-cat")
}

pub fn db_path() -> PathBuf {
    asset_dir().join(DB_FILENAME)
}
