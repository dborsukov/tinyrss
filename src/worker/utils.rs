use tokio::net::TcpStream;

pub fn get_app_dir() -> std::path::PathBuf {
    let config_dir = dirs::config_dir().unwrap();
    config_dir.join("tinyrss")
}

pub async fn is_online() -> bool {
    const ADDRS: [&str; 2] = ["clients3.google.com:80", "detectportal.firefox.com:80"];
    for addr in ADDRS {
        if (TcpStream::connect(addr).await).is_ok() {
            return true;
        }
    }
    false
}
