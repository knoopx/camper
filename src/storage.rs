use anyhow::Result;
use serde::{Deserialize, Serialize};
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

fn ui_state_path() -> PathBuf {
    config_dir().join("ui_state.json")
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiState {
    pub active_tab: Option<String>,
    pub search_query: Option<String>,
    pub discover_genre: Option<u32>,
    pub discover_subgenre: Option<u32>,
    pub discover_sort: Option<u32>,
    pub discover_format: Option<u32>,
    pub library_filter: Option<String>,
    pub volume: Option<f64>,
}

pub fn save_ui_state(state: &UiState) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    fs::write(ui_state_path(), serde_json::to_string(state)?)?;
    Ok(())
}

pub fn load_ui_state() -> UiState {
    fs::read_to_string(ui_state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
