#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let ef = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap()
        .add_directive("sqlx=warn".parse().unwrap());

    let ts = tracing_subscriber::fmt::fmt().with_env_filter(ef);

    ts.init();

    let native_options = eframe::NativeOptions {
        centered: true,
        resizable: false,
        always_on_top: false,
        icon_data: Some(load_icon()),
        initial_window_size: Some(egui::vec2(540.0, 800.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "Tinyrss",
        native_options,
        Box::new(|cc| Box::new(ui::TinyrssApp::new(cc))),
    )
}

fn load_icon() -> eframe::IconData {
    let image_bytes = include_bytes!("../icon.png");
    let image = image::load_from_memory(image_bytes).unwrap();
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_raw().clone();

    eframe::IconData {
        rgba: pixels,
        width: image.width(),
        height: image.height(),
    }
}
