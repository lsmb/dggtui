use config::{Config as Config_c, ConfigError, Environment, File};
use dirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub username: Option<String>,
    pub token: Option<String>,
    pub emotes: bool,
    pub autocomplete: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: None,
            token: None,
            emotes: false,
            autocomplete: false,
        }
    }
}

impl Config {
    pub async fn init() -> Result<Self, &'static str> {
        if make_config_folder().is_ok() {
            if config_exists() {
                let path: PathBuf = config_path().unwrap();
                let conf: Self = Config_c::builder()
                    .add_source(config::File::with_name(path.to_str().unwrap()))
                    .build()
                    .unwrap()
                    .try_deserialize()
                    .unwrap();
                Ok(conf)
            } else {
                Ok(Self {
                    ..Default::default()
                })
            }
        } else {
            Err("Something went wrong")
        }
    }
}

// pub async fn init() {
//     if make_config_folder().is_ok() {}
// }

fn make_config_folder() -> std::io::Result<()> {
    if cfg!(windows) {
        println!("this is windows");
    } else if cfg!(unix) {
        if let Some(home_dir) = dirs::home_dir() {
            let mut dggtui_conf: PathBuf = home_dir;
            dggtui_conf.push(".config");
            dggtui_conf.push("dggtui");
            if !dggtui_conf.exists() {
                fs::create_dir(dggtui_conf)?;
                // println!("Config directory created.")
            } else {
                // println!("Config direcotry exists.")
            }
        }
    }

    Ok(())
}

fn config_path() -> Option<PathBuf> {
    if cfg!(windows) {
        println!("this is windows");
    } else if cfg!(unix) {
        if let Some(home_dir) = dirs::home_dir() {
            let mut dggtui_conf: PathBuf = home_dir;
            dggtui_conf.push(".config");
            dggtui_conf.push("dggtui");
            dggtui_conf.push("dggtui.toml");
            return Some(dggtui_conf);
        }
    }
    None
}

fn config_exists() -> bool {
    if cfg!(windows) {
        println!("this is windows");
    } else if cfg!(unix) {
        if let Some(home_dir) = dirs::home_dir() {
            let mut dggtui_conf: PathBuf = home_dir;
            dggtui_conf.push(".config");
            dggtui_conf.push("dggtui");
            dggtui_conf.push("dggtui.toml");
            return dggtui_conf.exists();
        }
    }
    false
}
