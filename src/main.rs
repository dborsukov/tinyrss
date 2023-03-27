mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    tracing_subscriber::fmt::fmt().init();

    let native_options = eframe::NativeOptions {
        centered: true,
        resizable: false,
        always_on_top: true,
        initial_window_size: Some(egui::vec2(540.0, 800.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "Tinyrss",
        native_options,
        Box::new(|cc| Box::new(ui::TinyrssApp::new(cc))),
    )
}
