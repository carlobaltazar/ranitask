use crate::sequence::RemoteBinding;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_RECORD_VK: u16 = 0x77; // F8
pub const DEFAULT_STOP_VK: u16 = 0x7A;   // F11

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub record_vk: u16,
    #[serde(alias = "play_vk")]
    pub stop_vk: u16,
    pub loop_playback: bool,
    pub always_on_top: bool,
    #[serde(default = "default_remote_port")]
    pub remote_port: u16,
    #[serde(default)]
    pub remote_password: String,
    #[serde(default)]
    pub remote_auto_listen: bool,
    #[serde(default)]
    pub remote_hosts: Vec<String>,
    #[serde(default)]
    pub remote_bindings: Vec<RemoteBinding>,
    #[serde(default)]
    pub shuffle_queue: bool,
    #[serde(default)]
    pub queue_vk: Option<u16>,
    #[serde(default)]
    pub default_sequence: Option<String>,
}

fn default_remote_port() -> u16 { 9847 }

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            record_vk: DEFAULT_RECORD_VK,
            stop_vk: DEFAULT_STOP_VK,
            loop_playback: false,
            always_on_top: true,
            remote_port: default_remote_port(),
            remote_password: String::new(),
            remote_auto_listen: false,
            remote_hosts: Vec::new(),
            remote_bindings: Vec::new(),
            shuffle_queue: false,
            queue_vk: None,
            default_sequence: None,
        }
    }
}

fn config_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("ranitask");
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("[RaniTask] Failed to create config dir: {}", e);
    }
    dir.join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(json) = fs::read_to_string(&path) {
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json)
}
