use anyhow::Result;
use async_channel::unbounded;
use glib::MainContext;
use gtk4::{prelude::*, Application};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::{animation::Animator, database, input, ui};

pub struct BongoCatApp {
    counter: Arc<Mutex<u128>>,
}

impl BongoCatApp {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn initialize(&self) -> Result<u128> {
        let conn = database::connect().await?;
        database::initialize_schema(&conn).await?;

        let initial_count = database::read_counter(&conn).await?.unwrap_or(0);
        *self.counter.lock().unwrap() = initial_count;

        Ok(initial_count)
    }

    pub fn run(&self, app: Application, initial_count: u128) -> Result<()> {
        let counter = self.counter.clone();

        // Setup database writer thread
        let (db_sender, db_receiver) = mpsc::channel::<u128>();
        self.spawn_database_writer(db_receiver);

        app.connect_activate(move |app| {
            if let Err(e) = Self::setup_ui(app, initial_count, counter.clone(), db_sender.clone()) {
                eprintln!("Failed to setup UI: {}", e);
            }
        });

        app.run();
        Ok(())
    }

    fn setup_ui(
        app: &Application,
        initial_count: u128,
        counter: Arc<Mutex<u128>>,
        db_sender: mpsc::Sender<u128>,
    ) -> Result<()> {
        ui::load_custom_css()?;

        let window = ui::BongoWindow::new(app, initial_count)?;
        let assets = ui::AssetPaths::new()?;
        let animator = Animator::new(assets);

        let (key_sender, key_receiver) = unbounded();
        let _input_handles = input::start_input_monitoring(key_sender);

        MainContext::default().spawn_local(async move {
            while key_receiver.recv().await.is_ok() {
                animator.animate(&window.picture);

                let new_count = {
                    let mut guard = counter.lock().unwrap();
                    *guard += 1;
                    *guard
                };

                window.update_counter(new_count);

                if let Err(e) = db_sender.send(new_count) {
                    eprintln!("Database thread error: {}", e);
                }
            }
        });

        Ok(())
    }

    fn spawn_database_writer(&self, receiver: mpsc::Receiver<u128>) {
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to build Tokio runtime");

            rt.block_on(async move {
                let conn = match database::connect().await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to connect to database: {}", e);
                        return;
                    }
                };

                while let Ok(count) = receiver.recv() {
                    if let Err(e) = database::write_counter(&conn, count).await {
                        eprintln!("Database write error: {}", e);
                    }
                }
            });
        });
    }
}
