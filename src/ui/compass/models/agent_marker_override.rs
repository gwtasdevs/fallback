use anyhow::Result;
use indoc::indoc;

use crate::ui::compass::models::agent_marker_properties::AgentMarkerProperties;

use super::agent_marker_properties::MaybeAgentMarkerProperties;

pub struct AgentMarkerOverride {
  pub channel: String,
  pub name: String,
  pub priority: isize,
  pub properties: MaybeAgentMarkerProperties,
  pub when: Option<mlua::Function>,
}

impl mlua::FromLua for AgentMarkerOverride {
  fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
    let table = match value {
      mlua::Value::Table(table) => table,
      _ => {
        return Err(mlua::Error::FromLuaConversionError {
          from: value.type_name(),
          to: "AgentMarkerOverride".to_string(),
          message: Some("Table expected".to_string()),
        });
      }
    };

    let channel = table.get::<String>("channel")?;
    let name = table.get::<String>("name")?;
    let priority = table.get::<Option<isize>>("priority")?.unwrap_or(0);
    let properties = table.get::<MaybeAgentMarkerProperties>("properties")?;
    let when = table.get::<Option<mlua::Function>>("when")?;

    Ok(Self {
      channel,
      name,
      priority,
      properties,
      when,
    })
  }
}

impl AgentMarkerOverride {
  pub fn lua_format(&self) -> String {
    format!(
      indoc!(
        r#"
        {{
          channel = {channel},
          name = {name},
          priority = {priority},
          properties = {properties},
          when = {when},
        }}
      "#
      ),
      channel = format!(r#""{}""#, self.channel),
      name = format!(r#""{}""#, self.name),
      priority = self.priority,
      properties = self.properties.lua_format(),
      when = self
        .when
        .as_ref()
        .and_then(|when| when.info().source)
        .unwrap_or("nil".to_string()),
    )
  }

  pub fn to_properties(&self, agent_id: u32) -> Result<Option<(String, isize, AgentMarkerProperties)>> {
    if let Some(when) = self.when.as_ref() {
      if let Ok(result) = when.call::<bool>(agent_id) {
        if !result {
          return Ok(None);
        }
      } else {
        return Ok(None);
      }
    }

    match self.properties.to_properties(agent_id) {
      Ok(Some(properties)) => Ok(Some((self.channel.clone(), self.priority, properties))),
      Ok(None) => Ok(None),
      Err(err) => Err(err),
    }
  }
}
