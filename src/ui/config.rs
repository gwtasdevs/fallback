use serde::{Deserialize, Serialize};

use crate::utils::{CONFIG_PATH, write_file};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
  pub compass: CompassConfig,
  pub consumables: ConsumablesConfig,
  pub info: InfoConfig,
  pub window: WindowConfig,

  /** If true then windows cannot be moved or resized. */
  pub lock_windows: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct CompassConfig {
  pub enabled: bool,
  pub position: [f32; 2],
  pub size: f32,
}

impl Default for CompassConfig {
  fn default() -> Self {
    Self {
      enabled: true,
      position: [0., 0.],
      size: 200.,
    }
  }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ConsumablesConfig {
  pub enabled: bool,
  pub position: [f32; 2],
  pub size: [f32; 2],
}

impl Default for ConsumablesConfig {
  fn default() -> Self {
    Self {
      enabled: true,
      position: [0., 0.],
      size: [100., 100.],
    }
  }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct InfoConfig {
  pub collapsed: bool,
  pub enabled: bool,
  pub position: [f32; 2],
  pub size: [f32; 2],
}

impl Default for InfoConfig {
  fn default() -> Self {
    Self {
      collapsed: false,
      enabled: false,
      position: [0., 0.],
      size: [100., 100.],
    }
  }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
  pub collapsed: bool,
  pub position: [f32; 2],
  pub size: [f32; 2],
}

impl Default for WindowConfig {
  fn default() -> Self {
    Self {
      collapsed: false,
      position: [0., 0.],
      size: [300., 200.],
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
