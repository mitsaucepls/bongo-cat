mod animation;
mod app;
mod config;
mod database;
mod input;
mod ui;

use anyhow::Result;
use gtk4::Application;

use app::BongoCatApp;

fn main() -> Result<()> {
    let app = Application::builder()
        .application_id(config::APP_ID)
        .build();

    let bongo_app = BongoCatApp::new();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let initial_count = rt.block_on(bongo_app.initialize())?;

    bongo_app.run(app, initial_count)
}
