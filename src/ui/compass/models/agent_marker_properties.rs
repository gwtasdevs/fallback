use anyhow::{Result, bail};
use indoc::indoc;

use crate::ui::{
  common::{color_to_hex, hex_to_color},
  compass::models::shape::Shape,
};

pub enum MaybeAgentMarkerProperties {
  None,
  Function(mlua::Function),
  Struct(AgentMarkerProperties),
}

impl mlua::FromLua for MaybeAgentMarkerProperties {
  fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
    match value {
      mlua::Value::Function(func) => Ok(Self::Function(func)),
      mlua::Value::Table(table) => Ok(Self::Struct(AgentMarkerProperties::from_lua(
        mlua::Value::Table(table),
        lua,
      )?)),
      mlua::Value::Nil => Ok(Self::None),
      _ => Err(mlua::Error::FromLuaConversionError {
        from: value.type_name(),
        to: "AgentMarkerProperties".to_string(),
        message: Some("Table or Function expected".to_string()),
      }),
    }
  }
}

impl MaybeAgentMarkerProperties {
  pub fn lua_format(&self) -> String {
    match self {
      MaybeAgentMarkerProperties::Function(func) => func.info().source.unwrap_or("nil".to_string()),
      MaybeAgentMarkerProperties::None => "nil".to_string(),
      MaybeAgentMarkerProperties::Struct(properties) => properties.lua_format(),
    }
  }

  pub fn to_properties(&self, agent_id: u32) -> Result<Option<AgentMarkerProperties>> {
    match self {
      MaybeAgentMarkerProperties::Function(func) => {
        if let Ok(result) = func.call::<AgentMarkerProperties>(agent_id) {
          Ok(Some(result))
        } else {
          bail!("Error executing the `properties` function.");
        }
      }
      MaybeAgentMarkerProperties::None => Ok(None),
      MaybeAgentMarkerProperties::Struct(properties) => Ok(Some(properties.clone())),
    }
  }
}

#[derive(Clone, Default)]
pub struct AgentMarkerProperties {
  pub shape: Option<Shape>,
  pub size: Option<f32>,
  pub border_color: Option<[f32; 3]>,
  pub border_opacity: Option<f32>,
  pub border_thickness: Option<f32>,
  pub fill_color: Option<[f32; 3]>,
  pub fill_opacity: Option<f32>,
  pub draw_priority: Option<isize>,
}

impl mlua::FromLua for AgentMarkerProperties {
  fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
    let table = match value {
      mlua::Value::Table(table) => table,
      _ => {
        return Err(mlua::Error::FromLuaConversionError {
          from: value.type_name(),
          to: "AgentMarkerProperties".to_string(),
          message: Some("Table expected".to_string()),
        });
      }
    };

    let shape = table.get::<Option<Shape>>("shape")?;
    let size = table.get::<Option<f32>>("size")?;
    let border_color = table.get::<Option<u32>>("border_color")?.map(hex_to_color);
    let border_opacity = table.get::<Option<f32>>("border_opacity")?;
    let border_thickness = table.get::<Option<f32>>("border_thickness")?;
    let fill_color = table.get::<Option<u32>>("fill_color")?.map(hex_to_color);
    let fill_opacity = table.get::<Option<f32>>("fill_opacity")?;
    let draw_priority = table.get::<Option<isize>>("draw_priority")?;

    Ok(Self {
      shape,
      size,
      border_color,
      border_opacity,
      border_thickness,
      fill_color,
      fill_opacity,
      draw_priority,
    })
  }
}

impl AgentMarkerProperties {
  pub fn lua_format(&self) -> String {
    format!(
      indoc!(
        r#"
        {{
          shape = {shape},
          size = {size},
          border_color = {border_color},
          border_opacity = {border_opacity},
          fill_color = {fill_color},
          fill_opacity = {fill_opacity},
          draw_priority = {draw_priority},
        }}
      "#
      ),
      shape = self.shape.as_ref().map_or("nil".to_string(), |shape| format!(
        "Shape.{}",
        Into::<&str>::into(shape)
      )),
      size = self.size.map_or("nil".to_string(), |size| size.to_string()),
      border_color = self.border_color.map_or("nil".to_string(), color_to_hex),
      border_opacity = self
        .border_opacity
        .map_or("nil".to_string(), |opacity| opacity.clamp(0., 1.).to_string()),
      fill_color = self.fill_color.map_or("nil".to_string(), color_to_hex),
      fill_opacity = self
        .fill_opacity
        .map_or("nil".to_string(), |opacity| opacity.clamp(0., 1.).to_string()),
      draw_priority = self
        .draw_priority
        .map_or("nil".to_string(), |draw_priority| draw_priority.to_string()),
    )
  }

  pub fn merge(self, with: Self) -> Self {
    Self {
      shape: with.shape.or(self.shape),
      size: with.size.or(self.size),
      border_color: with.border_color.or(self.border_color),
      border_opacity: with.border_opacity.or(self.border_opacity),
      border_thickness: with.border_thickness.or(self.border_thickness),
      fill_color: with.fill_color.or(self.fill_color),
      fill_opacity: with.fill_opacity.or(self.fill_opacity),
      draw_priority: with.draw_priority.or(self.draw_priority),
    }
  }
}
