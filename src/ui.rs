use crate::config::{self, WINDOW_MARGIN_BOTTOM, WINDOW_MARGIN_RIGHT};
use gtk4::{
    gdk::Display, prelude::*, style_context_add_provider_for_display, CssProvider, Label, Picture,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::path::PathBuf;

pub struct BongoWindow {
    pub picture: Picture,
    pub counter_label: Label,
}

pub struct AssetPaths {
    pub idle: PathBuf,
    pub hit_left: PathBuf,
    pub hit_right: PathBuf,
}

impl BongoWindow {
    pub fn new(app: &Application, initial_count: u128) -> anyhow::Result<Self> {
        let assets = AssetPaths::new()?;

        let picture = Picture::for_filename(&assets.idle);
        let (width, height) = (picture.width(), picture.height());

        let window = ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .default_width(width)
            .default_height(height)
            .build();

        setup_layer_shell(&window);

        let counter_label = Label::new(Some(&initial_count.to_string()));

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.append(&counter_label);
        container.append(&picture);

        window.set_child(Some(&container));
        window.show();

        Ok(Self {
            picture,
            counter_label,
        })
    }

    pub fn update_counter(&self, count: u128) {
        self.counter_label.set_text(&count.to_string());
    }
}

impl AssetPaths {
    pub fn new() -> anyhow::Result<Self> {
        let asset_dir = config::asset_dir();

        let idle = asset_dir.join(config::IDLE_ASSET);
        let hit_left = asset_dir.join(config::HIT_LEFT_ASSET);
        let hit_right = asset_dir.join(config::HIT_RIGHT_ASSET);

        // Verify assets exist
        for (name, path) in [
            ("idle", &idle),
            ("hit_left", &hit_left),
            ("hit_right", &hit_right),
        ] {
            if !path.exists() {
                anyhow::bail!("Asset not found: {} at {:?}", name, path);
            }
        }

        Ok(Self {
            idle,
            hit_left,
            hit_right,
        })
    }
}

fn setup_layer_shell(window: &ApplicationWindow) {
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Right, true);
    window.set_decorated(false);
}

pub fn load_custom_css() -> anyhow::Result<()> {
    let css = format!(
        "window.background {{
            background-color: transparent;
            margin-bottom: {}px;
            margin-right: {}px;
        }}",
        WINDOW_MARGIN_BOTTOM, WINDOW_MARGIN_RIGHT
    );

    let provider = CssProvider::new();
    provider.load_from_data(&css);

    let display =
        Display::default().ok_or_else(|| anyhow::anyhow!("Could not connect to display"))?;

    style_context_add_provider_for_display(
        &display,
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    Ok(())
}
