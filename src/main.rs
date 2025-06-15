use std::{
    cell::Cell,
    env, fs,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
    u128,
};

use async_channel::{unbounded, Sender};
use evdev::{Device, EventSummary, KeyCode};
use glib::source::{idle_add_local, timeout_add_local};
use glib::{ControlFlow, MainContext};
use gtk4::{
    gdk::Display, prelude::*, style_context_add_provider_for_display, CssProvider, Label, Picture,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use limbo::{Connection, Value};

const ANIM_DURATION_MS: u32 = 150;

struct BongoCat {
    img: Arc<Mutex<Picture>>,
    hit_left_path: PathBuf,
    hit_right_path: PathBuf,
    idle_path: PathBuf,
    anim_ms: u32,
    next_left: Cell<bool>,
    counter: Mutex<u128>,
    counter_label: Label,
}

impl BongoCat {
    fn new(app: &Application, initial: u128) -> Self {
        let asset_dir = asset_dir();
        let hit_left_path = asset_dir.join("hit_left.png");
        let hit_right_path = asset_dir.join("hit_right.png");
        let idle_path = asset_dir.join("idle.png");

        let idle = Picture::for_filename(&idle_path);

        let (width, height) = (idle.width(), idle.height());

        let win = ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .default_width(width)
            .default_height(height)
            .build();

        win.init_layer_shell();
        win.set_layer(Layer::Overlay);
        win.set_anchor(Edge::Bottom, true);
        win.set_anchor(Edge::Right, true);
        win.set_decorated(false);

        let counter_label = Label::default();
        counter_label.set_text(&initial.to_string());

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.append(&counter_label);
        container.append(&idle);
        win.set_child(Some(&container));
        win.show();

        Self {
            img: Arc::new(Mutex::new(idle)),
            hit_left_path,
            hit_right_path,
            idle_path,
            anim_ms: ANIM_DURATION_MS,
            next_left: Cell::new(true),
            counter: Mutex::new(initial),
            counter_label: counter_label.clone(),
        }
    }

    fn increment_counter(&self) {
        let new_count = {
            let mut guard = self.counter.lock().unwrap();
            *guard += 1;
            *guard
        };
        let label = self.counter_label.clone();
        idle_add_local(move || {
            label.set_text(&new_count.to_string());
            ControlFlow::Break
        });
    }

    fn animate(&self) {
        let do_left = self.next_left.get();
        self.next_left.set(!do_left);

        let hit_path = if do_left {
            &self.hit_left_path
        } else {
            &self.hit_right_path
        };
        let idle_path = &self.idle_path;

        let delay = Duration::from_millis(self.anim_ms as u64);
        let img_arc = self.img.clone();

        let schedule = [
            (0 * delay, hit_path.clone()),
            (1 * delay, idle_path.clone()),
        ];

        for (offset, path) in schedule {
            let img = img_arc.clone();
            timeout_add_local(offset, move || {
                if let Ok(img_w) = img.lock() {
                    img_w.set_filename(Some(path.as_path()));
                }
                ControlFlow::Break
            });
        }
    }
}

fn asset_dir() -> PathBuf {
    if let Some(val) = env::var_os("BONGO_ASSETS") {
        return PathBuf::from(val);
    }

    dirs::config_dir()
        .expect("could not find a config directory")
        .join("bongo-cat")
}

fn start_key_listener(path: &str, sender: Sender<()>) -> thread::JoinHandle<()> {
    let dev_path = path.to_string();
    thread::spawn(move || {
        let mut dev = Device::open(&dev_path)
            .expect("Add your user to the ‘input’ group so you can read /dev/input/event*");
        loop {
            if let Ok(events) = dev.fetch_events() {
                for ev in events {
                    if let EventSummary::Key(_, _key, value) = ev.destructure() {
                        if value == 1 {
                            let _ = sender.try_send(());
                        }
                    }
                }
            }
        }
    })
}

fn load_custom_css() {
    let css = "
        window.background {
            background-color: transparent;
            margin-bottom: 93px;
            margin-right: 7px;
        }
    ";
    let provider = CssProvider::new();
    provider.load_from_data(css);
    style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

async fn connect_to_database() -> anyhow::Result<Connection> {
    let config_dir = asset_dir();
    let db_file = config_dir.join("sqlite.db");
    let db_path = db_file.to_str().unwrap();
    let db = limbo::Builder::new_local(db_path).build().await?;

    Ok(db.connect()?)
}

async fn write_counter_into_db(conn: Connection, counter: u128) -> anyhow::Result<()> {
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS counter (
            id    INTEGER PRIMARY KEY,
            count TEXT NOT NULL
        )
        ",
        (),
    )
    .await?;

    let counter_str = counter.to_string();

    let rows_updated = conn
        .execute(
            "UPDATE counter SET count = ?1 WHERE id = 1",
            [counter_str.as_str()],
        )
        .await?;

    if rows_updated == 0 {
        conn.execute(
            "INSERT INTO counter (id, count) VALUES (1, ?1)",
            [counter_str.as_str()],
        )
        .await?;
    }

    Ok(())
}

async fn read_counter_from_db(conn: Connection) -> anyhow::Result<Option<u128>> {
    let mut rows = conn
        .query("SELECT count FROM counter WHERE id = 1", ())
        .await?;

    if let Some(row) = rows.next().await? {
        let v = row.get_value(0)?;

        if let Value::Text(s) = v {
            let n = s.parse::<u128>()?;
            Ok(Some(n))
        } else {
            anyhow::bail!("expected TEXT for counter, got {:?}", v);
        }
    } else {
        Ok(None)
    }
}

fn main() -> anyhow::Result<()> {
    let (db_signal, db_receive) = mpsc::channel::<u128>();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build Tokio runtime");
        rt.block_on(async move {
            let conn = connect_to_database()
                .await
                .expect("failed to connect to database");
            while let Ok(count) = db_receive.recv() {
                if let Err(err) = write_counter_into_db(conn.clone(), count).await {
                    eprintln!("DB write error: {:?}", err);
                }
            }
        });
    });

    // std::env::set_var("GTK_THEME", "Adwaita");
    let app = Application::builder()
        .application_id("com.example.BongoCat")
        .build();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let conn = rt.block_on(connect_to_database())?;
    let initial = rt
        .block_on(read_counter_from_db(conn.clone()))?
        .unwrap_or(0);

    app.connect_activate(move |app| {
        load_custom_css();
        let bongo = BongoCat::new(app, initial);
        let (key_sender, key_receiver) = unbounded::<()>();

        for entry in fs::read_dir("/dev/input").expect("failed to read /dev/input") {
            let path = match entry {
                Ok(e) => e.path(),
                Err(_) => continue,
            };
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("event") {
                    if let Ok(dev) = Device::open(&path) {
                        if dev
                            .supported_keys()
                            .map_or(false, |keys| keys.contains(KeyCode::KEY_A))
                        {
                            start_key_listener(&path.to_string_lossy(), key_sender.clone());
                        }
                    }
                }
            }
        }

        let db_signal_for_async = db_signal.clone();
        MainContext::default().spawn_local(async move {
            while key_receiver.recv().await.is_ok() {
                bongo.animate();
                bongo.increment_counter();
                let current = *bongo.counter.lock().unwrap();
                if let Err(_) = db_signal_for_async.send(current) {
                    eprintln!("DB thread has shut down");
                }
            }
        });
    });

    app.run();
    Ok(())
}
