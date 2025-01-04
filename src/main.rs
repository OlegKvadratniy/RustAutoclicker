use rdev::{simulate, listen, EventType, Key as RdevKey, Button, Event};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use rand::Rng;
use eframe::{egui, App, NativeOptions};
use enigo::{Enigo, Key as EnigoKey, Keyboard};

fn click_mouse() {
    if let Err(e) = simulate(&EventType::ButtonPress(Button::Left)) {
        eprintln!("Failed to simulate mouse press: {:?}", e);
    }
    thread::sleep(Duration::from_millis(50));
    if let Err(e) = simulate(&EventType::ButtonRelease(Button::Left)) {
        eprintln!("Failed to simulate mouse release: {:?}", e);
    }
}

fn regular_clicker(interval: Arc<Mutex<u64>>, running: Arc<Mutex<bool>>, tx: mpsc::Sender<String>) {
    let mut last_click = Instant::now();
    while *running.lock().unwrap() {
        let interval = *interval.lock().unwrap();
        let interval = if interval < 1 { 1 } else { interval };
        if last_click.elapsed().as_millis() >= interval as u128 {
            click_mouse();
            tx.send("Click!".to_string()).unwrap();
            last_click = Instant::now();
        }
    }
}

fn jitter_clicker(min_interval: Arc<Mutex<u64>>, max_interval: Arc<Mutex<u64>>, jitter: Arc<Mutex<u64>>, running: Arc<Mutex<bool>>, tx: mpsc::Sender<String>) {
    let mut rng = rand::thread_rng();
    let mut last_click = Instant::now();
    while *running.lock().unwrap() {
        let min_interval = *min_interval.lock().unwrap();
        let max_interval = *max_interval.lock().unwrap();
        let jitter = *jitter.lock().unwrap();
        let min_interval = if min_interval < 1 { 1 } else { min_interval };
        let interval = rng.gen_range(min_interval..=max_interval) + rng.gen_range(0..=jitter);
        if last_click.elapsed().as_millis() >= interval as u128 {
            click_mouse();
            tx.send(format!("Click with jitter: {} ms", interval)).unwrap();
            last_click = Instant::now();
        }
    }
}

struct AutoClickerApp {
    interval: Arc<Mutex<u64>>,
    min_interval: Arc<Mutex<u64>>,
    max_interval: Arc<Mutex<u64>>,
    jitter: Arc<Mutex<u64>>,
    logs: Arc<Mutex<Vec<String>>>,
    running_regular: Arc<Mutex<bool>>,
    running_jitter: Arc<Mutex<bool>>,
    tx: mpsc::Sender<String>,
    rx: Arc<Mutex<mpsc::Receiver<String>>>,
    is_running_regular: Arc<Mutex<bool>>,
    is_running_jitter: Arc<Mutex<bool>>,
    enable_jitter: Arc<Mutex<bool>>,
    f9_pressed: Arc<Mutex<bool>>, // Add field to store F9 key state
}

impl eframe::App for AutoClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Autoclicker");

            ui.separator();

            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Interval (ms):").size(18.0));
                    ui.add(
                        egui::DragValue::new(&mut *self.interval.lock().unwrap())
                            .range(1..=u64::MAX)
                            .clamp_existing_to_range(true),
                    );
                });

                ui.add_space(10.0);

                let button_text = if *self.is_running_regular.lock().unwrap() || *self.is_running_jitter.lock().unwrap() { "STOP" } else { "START[F9]" };
                let button_style = egui::Button::new(button_text)
                    .min_size(egui::Vec2::new(250.0, 100.0))
                    .rounding(egui::Rounding::same(10.0))
                    .wrap();

                if ui.add(button_style).clicked() {
                    self.toggle_clicker();
                }

                ui.add_space(20.0);

                ui.checkbox(&mut *self.enable_jitter.lock().unwrap(), "Enable Jitter Mode");

                ui.add_space(10.0);

                if *self.enable_jitter.lock().unwrap() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Min Interval (ms):").size(18.0));
                        ui.add(
                            egui::DragValue::new(&mut *self.min_interval.lock().unwrap())
                                .range(1..=u64::MAX)
                                .clamp_existing_to_range(true),
                        );

                        ui.add_space(5.0);

                        ui.label(egui::RichText::new("Max Interval (ms):").size(18.0));
                        ui.add(
                            egui::DragValue::new(&mut *self.max_interval.lock().unwrap())
                                .range(1..=u64::MAX)
                                .clamp_existing_to_range(true),
                        );

                        ui.add_space(5.0);

                        ui.label(egui::RichText::new("Jitter (ms):").size(18.0));
                        ui.add(
                            egui::DragValue::new(&mut *self.jitter.lock().unwrap())
                                .range(0..=u64::MAX)
                                .clamp_existing_to_range(true),
                        );
                    });
                }
            });

            ui.separator();

            ui.label("Logs:");
            egui::Frame::none()
                .fill(egui::Color32::from_gray(60))
                .rounding(egui::Rounding::same(10.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let logs = self.logs.lock().unwrap();
                            for log in logs.iter() {
                                ui.colored_label(egui::Color32::WHITE, log);
                            }
                        });
                });
        });

        while let Ok(log) = self.rx.lock().unwrap().try_recv() {
            self.logs.lock().unwrap().push(log);
        }

        if *self.f9_pressed.lock().unwrap() {
            self.toggle_clicker();
            *self.f9_pressed.lock().unwrap() = false; // Reset F9 key state
        }
    }
}

impl AutoClickerApp {
    fn toggle_clicker(&mut self) {
        if *self.is_running_regular.lock().unwrap() || *self.is_running_jitter.lock().unwrap() {
            if *self.is_running_regular.lock().unwrap() {
                let mut running = self.running_regular.lock().unwrap();
                *running = false;
                self.logs.lock().unwrap().push("Regular mode stopped.".to_string());
                let mut is_running = self.is_running_regular.lock().unwrap();
                *is_running = false;
            }
            if *self.is_running_jitter.lock().unwrap() {
                let mut running = self.running_jitter.lock().unwrap();
                *running = false;
                self.logs.lock().unwrap().push("Jitter mode stopped.".to_string());
                let mut is_running = self.is_running_jitter.lock().unwrap();
                *is_running = false;
            }
        } else {
            let running = Arc::clone(&self.running_regular);
            *running.lock().unwrap() = true;
            let interval = Arc::clone(&self.interval);
            let tx = self.tx.clone();
            let logs = Arc::clone(&self.logs);

            if *self.enable_jitter.lock().unwrap() {
                let min_interval = Arc::clone(&self.min_interval);
                let max_interval = Arc::clone(&self.max_interval);
                let jitter = Arc::clone(&self.jitter);
                let running_jitter = Arc::clone(&self.running_jitter);
                *running_jitter.lock().unwrap() = true;
                thread::spawn(move || {
                    jitter_clicker(min_interval, max_interval, jitter, Arc::clone(&running_jitter), tx);
                    logs.lock().unwrap().push("Jitter mode started.".to_string());
                });
                let mut is_running = self.is_running_jitter.lock().unwrap();
                *is_running = true;
            } else {
                thread::spawn(move || {
                    regular_clicker(interval, Arc::clone(&running), tx);
                    logs.lock().unwrap().push("Regular mode started.".to_string());
                });
                let mut is_running = self.is_running_regular.lock().unwrap();
                *is_running = true;
            }
        }
    }
}

#[derive(Clone)]
struct AutoClickerAppWrapper {
    app: Arc<Mutex<AutoClickerApp>>,
}

impl eframe::App for AutoClickerAppWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.lock().unwrap().update(ctx, frame);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, rx) = mpsc::channel();
    let app = Arc::new(Mutex::new(AutoClickerApp {
        interval: Arc::new(Mutex::new(100)),
        min_interval: Arc::new(Mutex::new(100)),
        max_interval: Arc::new(Mutex::new(200)),
        jitter: Arc::new(Mutex::new(50)),
        logs: Arc::new(Mutex::new(vec![])),
        running_regular: Arc::new(Mutex::new(false)),
        running_jitter: Arc::new(Mutex::new(false)),
        tx,
        rx: Arc::new(Mutex::new(rx)),
        is_running_regular: Arc::new(Mutex::new(false)),
        is_running_jitter: Arc::new(Mutex::new(false)),
        enable_jitter: Arc::new(Mutex::new(false)),
        f9_pressed: Arc::new(Mutex::new(false)), // Инициализация состояния клавиши F9
    }));

    // Запуск потока для прослушивания событий клавиатуры
    let app_clone = Arc::clone(&app);
    thread::spawn(move || {
        listen(move |event: Event| {
            if let EventType::KeyPress(key) = event.event_type {
                if key == RdevKey::F9 {
                    let app_lock = app_clone.lock().unwrap();
                    let mut f9_pressed = app_lock.f9_pressed.lock().unwrap();
                    *f9_pressed = true; // Обработка нажатия F9
                }
            }
        }).expect("Failed to listen to key events");
    });

    let native_options = NativeOptions::default();
    let app_wrapper = AutoClickerAppWrapper {
        app: Arc::clone(&app),
    };

    eframe::run_native(
        "AutoClicker",
        native_options,
        Box::new(|_cc| {
            let app_wrapper_clone = app_wrapper.clone();
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Box::new(app_wrapper_clone) as Box<dyn App>)
        })
    );

    Ok(())
}








