use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use indoc::indoc;

use crate::ui::common::lua::{self, update_lua_values};
use crate::ui::compass::models::agent_marker_properties::AgentMarkerProperties;
use crate::utils::write_file;

use super::agent_marker_override::AgentMarkerOverride;
use super::marker::Marker;

type AgentMarkerWithChannel = (Option<String>, AgentMarkerProperties);
type AgentMarkerOverrideWithPriority = (isize, AgentMarkerProperties);

pub struct MarkerGroup {
  pub enabled: bool,
  pub name: String,
  pub path: PathBuf,
  pub markers: Vec<Marker>,
  pub overrides: Vec<AgentMarkerOverride>,
  pub when: Option<mlua::Function>,

  pub child_groups: Vec<MarkerGroup>,

  _lua: Arc<mlua::Lua>,
}

impl MarkerGroup {
  const ROOT_NAME: &str = "marker_group";
  const FIELD_NAMES: [&str; 4] = ["enabled", "name", "markers", "overrides"];

  pub fn new(lua: Arc<mlua::Lua>, path: PathBuf) -> Self {
    let mut child_groups = vec![];

    let file_name = path
      .file_name()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or("_.lua".to_string());

    if file_name == "_.lua"
      && let Some(folder_path) = path.parent()
      && let Ok(entries) = std::fs::read_dir(folder_path)
    {
      for entry in entries {
        if let Ok(entry) = entry
          && let Ok(file_type) = entry.file_type()
        {
          if file_type.is_dir() {
            child_groups.push(Self::new(lua.clone(), entry.path().join("_.lua")));
          } else if entry.file_name().to_string_lossy().ends_with(".lua") {
            if entry.file_name() == "_meta.lua" || entry.file_name() == "_.lua" {
              continue;
            }
            child_groups.push(Self::new(lua.clone(), entry.path()));
          }
        }
      }
    }

    child_groups.sort_by(|a, b| a.name.cmp(&b.name));

    let name = path
      .file_stem()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or("<unnamed>".to_string());

    if let Ok(code) = std::fs::read_to_string(&path) {
      if let Ok(env) = lua::execute_script(&lua, &path, &code) {
        if let Ok(table) = env.get::<mlua::Table>(Self::ROOT_NAME) {
          if let Ok(mut marker_group) = Self::from_table(lua.clone(), &table, path.clone(), name.clone()) {
            marker_group.child_groups = child_groups;
            return marker_group;
          } else {
            log::error!(
              "Couldn't parse '{}' definition in script for marker group: {:?}",
              Self::ROOT_NAME,
              &path
            );
          }
        } else {
          log::error!(
            "Couldn't find '{}' definition in script for marker group: {:?}",
            Self::ROOT_NAME,
            &path
          );
        }
      } else {
        log::error!("Couldn't load script for marker group: {:?}", &path);
      }
    }

    Self {
      enabled: true,
      name,
      path,
      markers: vec![],
      overrides: vec![],
      when: None,
      child_groups,
      _lua: lua,
    }
  }

  fn from_table(lua: Arc<mlua::Lua>, table: &mlua::Table, path: PathBuf, name: String) -> Result<Self> {
    let enabled = table.get::<Option<bool>>("enabled")?.unwrap_or(true);
    let name = table.get::<Option<String>>("name")?.unwrap_or(name);
    let markers = table.get::<Option<Vec<Marker>>>("markers")?.unwrap_or(vec![]);
    let overrides = table
      .get::<Option<Vec<AgentMarkerOverride>>>("overrides")?
      .unwrap_or(vec![]);
    let when = table.get::<Option<mlua::Function>>("when")?;

    Ok(Self {
      enabled,
      name,
      path,
      markers,
      overrides,
      when,
      child_groups: vec![],
      _lua: lua,
    })
  }

  #[expect(unused)]
  pub fn update_on_disk(&self) -> Result<()> {
    std::fs::create_dir_all(&self.path)?;

    let updated_contents = if let Ok(contents) = std::fs::read_to_string(&self.path) {
      Some(update_lua_values(
        contents,
        Self::ROOT_NAME,
        &Self::FIELD_NAMES,
        |field_name| {
          // TODO: omit fields that have default values
          match field_name {
            "enabled" => Ok(if self.enabled { "true" } else { "false" }.to_owned()),
            "name" => Ok(format!(r#""{}""#, self.name)),
            "markers" => Ok(format!(
              "{{\n{}\n}}",
              self
                .markers
                .iter()
                .map(|marker| marker
                  .lua_format()
                  .lines()
                  .map(|line| format!("  {}", line))
                  .collect::<Vec<_>>()
                  .join("\n"))
                .map(|marker| format!("{},", marker))
                .collect::<Vec<_>>()
                .join("\n")
            )),
            "overrides" => Ok(format!(
              "{{\n{}\n}}",
              self
                .overrides
                .iter()
                .map(|r#override| r#override
                  .lua_format()
                  .lines()
                  .map(|line| format!("  {}", line))
                  .collect::<Vec<_>>()
                  .join("\n"))
                .map(|r#override| format!("{},", r#override))
                .collect::<Vec<_>>()
                .join("\n")
            )),
            _ => bail!("Invalid field name '{}'.", field_name),
          }
        },
      )?)
    } else {
      let mut fields = vec![];
      if self
        .path
        .file_stem()
        .map(|name| name.to_string_lossy() != self.name)
        .unwrap_or(true)
      {
        fields.push(format!(r#"  name = "{}","#, self.name));
      }
      if !self.enabled {
        fields.push("  enabled = false,".to_string());
      }
      if !self.markers.is_empty() {
        fields.push("  markers = ,".to_string());
      }
      if !self.overrides.is_empty() {
        fields.push("  overrides = ,".to_string());
      }
      if self.when.is_some() {
        fields.push("  when = ,".to_string());
      }

      if fields.is_empty() {
        None
      } else {
        Some(format!(
          indoc!(
            r#"
            ---@type ScriptGroup
            {root_name} = {{
            {fields}
            }}
          "#
          ),
          root_name = Self::ROOT_NAME,
          fields = fields.join("\n"),
        ))
      }
    };

    if let Some(updated_contents) = updated_contents {
      write_file(&self.path, updated_contents)?;
    }

    Ok(())
  }

  pub fn get_agent_markers(
    &self,
    agent_id: u32,
  ) -> (
    Vec<AgentMarkerWithChannel>,
    HashMap<String, Vec<AgentMarkerOverrideWithPriority>>,
  ) {
    if !self.enabled {
      return (vec![], HashMap::new());
    }

    if let Some(when) = self.when.as_ref() {
      match when.call::<bool>(()) {
        Ok(result) => {
          if !result {
            return (vec![], HashMap::new());
          }
        }
        Err(err) => {
          log_when_error(err, None, &self.path);
          return (vec![], HashMap::new());
        }
      }
    }

    let mut markers = self
      .markers
      .iter()
      .filter_map(|marker| match marker {
        Marker::Agent {
          channel,
          name,
          properties,
          when,
        } => {
          if let Some(when) = when {
            match when.call::<bool>(agent_id) {
              Ok(result) => {
                if !result {
                  return None;
                }
              }
              Err(err) => {
                log_when_error(err, Some(name), &self.path);
                return None;
              }
            }
          }

          match properties.to_properties(agent_id) {
            Ok(Some(result)) => Some((channel.to_owned(), result)),
            Ok(None) => None,
            Err(err) => {
              log::error!(
                "Error resolving the agent marker '{}' of marker group '{:?}'.\n{:?}",
                name,
                self.path,
                err
              );
              None
            }
          }
        }
        _ => None,
      })
      .collect::<Vec<_>>();

    let mut overrides = self
      .overrides
      .iter()
      .filter_map(|marker_override| match marker_override.to_properties(agent_id) {
        Ok(Some(result)) => Some(result),
        Ok(None) => None,
        Err(err) => {
          log::error!(
            "Error resolving the agent marker override '{}' of marker group '{:?}'.\n{:?}",
            marker_override.name,
            self.path,
            err,
          );
          None
        }
      })
      .fold(
        HashMap::<String, Vec<(isize, AgentMarkerProperties)>>::new(),
        |mut acc, (channel, priority, properties)| {
          acc.entry(channel).or_default().push((priority, properties));
          acc
        },
      );

    for group in self.child_groups.iter() {
      let (new_markers, new_overrides) = group.get_agent_markers(agent_id);
      markers.extend(new_markers);

      for (channel, overrides_for_channel) in new_overrides {
        overrides.entry(channel).or_default().extend(overrides_for_channel);
      }
    }

    (markers, overrides)
  }

  pub fn resolve_agent_markers(&self, agent_id: u32) -> Vec<AgentMarkerProperties> {
    if !self.enabled {
      return vec![];
    }

    let (markers, overrides) = self.get_agent_markers(agent_id);

    let overrides = overrides
      .into_iter()
      .map(|(channel, mut entries)| {
        entries.sort_by_key(|(priority, _)| *priority);
        let result = entries
          .into_iter()
          .map(|(_, properties)| properties)
          .reduce(|acc, properties| acc.merge(properties));
        (channel, result)
      })
      .collect::<HashMap<String, Option<AgentMarkerProperties>>>();

    markers
      .into_iter()
      .map(|(channel, properties)| {
        if let Some(channel) = channel
          && let Some(Some(o)) = overrides.get(&channel)
        {
          properties.merge(o.clone())
        } else {
          properties
        }
      })
      .collect()
  }

  pub fn resolve_static_markers(&self) -> Vec<&Marker> {
    if !self.enabled {
      return vec![];
    }

    let mut markers = self
      .markers
      .iter()
      .filter(|marker| match marker {
        Marker::Circle { name, when, .. } => {
          if let Some(when) = when {
            match when.call::<bool>(()) {
              Ok(result) => {
                if !result {
                  return false;
                }
              }
              Err(err) => {
                log_when_error(err, Some(name), &self.path);
                return false;
              }
            }
          }

          true
        }
        Marker::Line { name, when, .. } => {
          if let Some(when) = when {
            match when.call::<bool>(()) {
              Ok(result) => {
                if !result {
                  return false;
                }
              }
              Err(err) => {
                log_when_error(err, Some(name), &self.path);
                return false;
              }
            }
          }

          true
        }
        _ => false,
      })
      .collect::<Vec<_>>();

    for group in self.child_groups.iter() {
      markers.extend(group.resolve_static_markers());
    }

    markers
  }
}

fn log_when_error(err: mlua::Error, name: Option<&str>, path: &Path) {
  let name = if let Some(name) = name {
    format!(" of marker '{}'", name)
  } else {
    String::new()
  };
  if let mlua::Error::RuntimeError(ref msg) = err
    && msg.contains("attempt to yield")
  {
    log::error!(
      "Error executing the `when` function{} in marker group '{:?}': {}.
      Marker functions run per-frame, and cannot contain long-running function calls such as gw.util.wait().",
      name,
      path,
      msg,
    );
  } else {
    log::error!(
      "Error executing the `when` function{} in marker group '{:?}': {}.",
      name,
      path,
      err
    );
  }
}
