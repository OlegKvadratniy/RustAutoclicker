use std::thread;
use std::time::{Instant};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use rand::Rng;
use eframe::{egui, App, NativeOptions};

// Function for performing clicks in regular mode
fn regular_clicker(interval: u64, running: Arc<Mutex<bool>>, tx: mpsc::Sender<String>) {
    let mut last_click = Instant::now();
    while *running.lock().unwrap() {
        let interval = if interval < 1 { 1 } else { interval }; // Minimum interval value 1 ms
        if last_click.elapsed().as_millis() >= interval as u128 {
            tx.send("Click!".to_string()).unwrap();
            last_click = Instant::now();
        }
    }
}

// Function for performing clicks in jitter mode
fn jitter_clicker(min_interval: u64, max_interval: u64, jitter: u64, running: Arc<Mutex<bool>>, tx: mpsc::Sender<String>) {
    let mut rng = rand::thread_rng();
    let mut last_click = Instant::now();
    while *running.lock().unwrap() {
        let min_interval = if min_interval < 1 { 1 } else { min_interval }; // Minimum interval value 1 ms
        let interval = rng.gen_range(min_interval..=max_interval) + rng.gen_range(0..=jitter);
        if last_click.elapsed().as_millis() >= interval as u128 {
            tx.send(format!("Click with jitter: {} ms", interval)).unwrap();
            last_click = Instant::now();
        }
    }
}

struct AutoClickerApp {
    interval: u64,
    min_interval: u64,
    max_interval: u64,
    jitter: u64,
    logs: Arc<Mutex<Vec<String>>>,
    running_regular: Arc<Mutex<bool>>,
    running_jitter: Arc<Mutex<bool>>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
    is_running_regular: bool,
    is_running_jitter: bool,
    enable_jitter: bool,
}

impl App for AutoClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Autoclicker");

            ui.separator();

            ui.vertical_centered(|ui| {
                ui.add_space(20.0); // Top padding

                // Input for Interval
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Interval (ms):").size(18.0));
                    ui.add(
                        egui::DragValue::new(&mut self.interval)
                            .range(1..=u64::MAX)
                            .clamp_existing_to_range(true),
                    ); // Increased size
                });

                ui.add_space(10.0); // Space between input and start button

                // Start/Stop button
                let button_text = if self.is_running_regular || self.is_running_jitter { "STOP" } else { "START" };
                let button_style = egui::Button::new(button_text)
                    .min_size(egui::Vec2::new(250.0, 100.0)) // Increased button size
                    .rounding(egui::Rounding::same(10.0))
                    .wrap();

                if ui.add(button_style).clicked() {
                    if self.is_running_regular || self.is_running_jitter {
                        if self.is_running_regular {
                            let mut running = self.running_regular.lock().unwrap();
                            *running = false;
                            self.logs.lock().unwrap().push("Regular mode stopped.".to_string());
                            self.is_running_regular = false;
                        }
                        if self.is_running_jitter {
                            let mut running = self.running_jitter.lock().unwrap();
                            *running = false;
                            self.logs.lock().unwrap().push("Jitter mode stopped.".to_string());
                            self.is_running_jitter = false;
                        }
                    } else {
                        let running = Arc::clone(&self.running_regular);
                        *running.lock().unwrap() = true;
                        let interval = self.interval;
                        let tx = self.tx.clone();
                        let logs = Arc::clone(&self.logs);

                        if self.enable_jitter {
                            let min_interval = self.min_interval;
                            let max_interval = self.max_interval;
                            let jitter = self.jitter;
                            let running_jitter = Arc::clone(&self.running_jitter);
                            *running_jitter.lock().unwrap() = true;
                            thread::spawn(move || {
                                jitter_clicker(min_interval, max_interval, jitter, Arc::clone(&running_jitter), tx);
                                logs.lock().unwrap().push("Jitter mode started.".to_string());
                            });
                            self.is_running_jitter = true;
                        } else {
                            thread::spawn(move || {
                                regular_clicker(interval, Arc::clone(&running), tx);
                                logs.lock().unwrap().push("Regular mode started.".to_string());
                            });
                            self.is_running_regular = true;
                        }
                    }
                }

                ui.add_space(20.0); // Space between start button and jitter toggle

                // Jitter Mode Toggle and Inputs
                ui.checkbox(&mut self.enable_jitter, "Enable Jitter Mode");

                ui.add_space(10.0); // Space between checkbox and jitter settings

                if self.enable_jitter {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Min Interval (ms):").size(18.0));
                        ui.add(
                            egui::DragValue::new(&mut self.min_interval)
                                .range(1..=u64::MAX)
                                .clamp_existing_to_range(true),
                        ); // Increased size

                        ui.add_space(5.0); // Space between inputs

                        ui.label(egui::RichText::new("Max Interval (ms):").size(18.0));
                        ui.add(
                            egui::DragValue::new(&mut self.max_interval)
                                .range(1..=u64::MAX)
                                .clamp_existing_to_range(true),
                        ); // Increased size

                        ui.add_space(5.0); // Space between inputs

                        ui.label(egui::RichText::new("Jitter (ms):").size(18.0));
                        ui.add(
                            egui::DragValue::new(&mut self.jitter)
                                .range(0..=u64::MAX)
                                .clamp_existing_to_range(true),
                        ); // Increased size
                    });
                }
            });

            ui.separator();

            // Logs
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

        while let Ok(log) = self.rx.try_recv() {
            self.logs.lock().unwrap().push(log);
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::channel();
    let app = AutoClickerApp {
        interval: 1000,
        min_interval: 500,
        max_interval: 1500,
        jitter: 100,
        logs: Arc::new(Mutex::new(Vec::new())),
        running_regular: Arc::new(Mutex::new(false)),
        running_jitter: Arc::new(Mutex::new(false)),
        tx,
        rx,
        is_running_regular: false,
        is_running_jitter: false,
        enable_jitter: false,
    };

    let native_options = NativeOptions::default();
    eframe::run_native(
        "Autoclicker",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}
