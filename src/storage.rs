use anyhow::Result;
use std::fs;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("camper")
}

fn cookies_path() -> PathBuf {
    config_dir().join("cookies")
}

pub fn save_cookies(cookies: &str) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    fs::write(cookies_path(), cookies)?;
    Ok(())
}

pub fn load_cookies() -> Option<String> {
    fs::read_to_string(cookies_path()).ok()
}

pub fn clear_cookies() {
    let _ = fs::remove_file(cookies_path());
}
