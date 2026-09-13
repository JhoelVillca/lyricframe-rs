use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use windows::Win32::System::SystemInformation::GetComputerNameExW;
use windows::Win32::System::SystemInformation::ComputerNameDnsHostname;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub device_id: String,
}

fn get_hostname() -> String {
    let mut buffer = [0u16; 256];
    let mut size = buffer.len() as u32;
    unsafe {
        if GetComputerNameExW(ComputerNameDnsHostname, windows::core::PWSTR(buffer.as_mut_ptr()), &mut size).is_ok() {
            if let Ok(hostname) = String::from_utf16(&buffer[..(size as usize)]) {
                return hostname;
            }
        }
    }
    "unknown-host".to_string()
}

pub fn get_or_create_device_id() -> String {
    let config_path = get_config_path();

    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<Config>(&content) {
            return config.device_id;
        }
    }

    let hostname = get_hostname();
    let uuid = Uuid::new_v4().to_string();
    let device_id = format!("{}-{}", hostname, uuid);

    let config = Config {
        device_id: device_id.clone(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&config) {
        let _ = fs::write(&config_path, json);
    }

    device_id
}

pub fn setup_autostart() {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_str) = exe_path.to_str() {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(run_key) = hkcu.open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                KEY_WRITE,
            ) {
                let _ = run_key.set_value("LyricFrameCollector", &exe_str);
            }
        }
    }
}

fn get_config_path() -> PathBuf {
    if let Ok(mut exe_path) = env::current_exe() {
        exe_path.pop();
        exe_path.push("config.json");
        exe_path
    } else {
        PathBuf::from("config.json")
    }
}
