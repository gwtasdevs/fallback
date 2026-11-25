use serde::{Deserialize, Serialize};

use crate::utils::{CONFIG_PATH, write_file};

#[derive(Serialize, Deserialize)]
pub struct Config {
  pub compass: CompassConfig,
  pub info: InfoConfig,
  pub window: WindowConfig,

  /** If true then windows cannot be moved or resized. */
  #[serde(default)]
  pub lock_windows: bool,
}

#[derive(Serialize, Deserialize)]
pub struct CompassConfig {
  pub enabled: bool,
  pub position: [f32; 2],
  pub size: f32,
}

#[derive(Serialize, Deserialize)]
pub struct InfoConfig {
  pub enabled: bool,
  pub position: [f32; 2],
  pub size: [f32; 2],
}

#[derive(Serialize, Deserialize)]
pub struct WindowConfig {
  pub position: [f32; 2],
  pub size: [f32; 2],
}

impl Default for Config {
  fn default() -> Self {
    Self {
      compass: CompassConfig {
        enabled: true,
        position: [0.0, 0.0],
        size: 100.0,
      },
      info: InfoConfig {
        enabled: false,
        position: [0.0, 0.0],
        size: [100.0, 100.0],
      },
      window: WindowConfig {
        position: [0.0, 0.0],
        size: [300.0, 200.0],
      },
      lock_windows: false,
    }
  }
}

impl Config {
  pub fn load() -> Self {
    let path = CONFIG_PATH.join("main.yaml");
    std::fs::read_to_string(&path)
      .ok()
      .and_then(|s| {
        if let Ok(config) = serde_yaml::from_str(&s) {
          Some(config)
        } else {
          log::error!("Couldn't load '{:?}', invalid yaml.", path);
          None
        }
      })
      .unwrap_or_default()
  }

  pub fn save(&self) {
    if let Ok(s) = serde_yaml::to_string(self)
      && std::fs::create_dir_all(&*CONFIG_PATH).is_ok()
    {
      let _ = write_file(CONFIG_PATH.join("main.yaml"), s);
    }
  }
}
