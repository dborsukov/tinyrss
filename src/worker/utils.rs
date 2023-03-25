pub fn get_app_dir() -> std::path::PathBuf {
    let config_dir = dirs::config_dir().unwrap();
    config_dir.join("tinyrss")
}
