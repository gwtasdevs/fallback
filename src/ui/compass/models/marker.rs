use indoc::indoc;

use super::agent_marker_properties::MaybeAgentMarkerProperties;
use crate::ui::common::{color_to_hex, hex_to_color};

pub enum Marker {
  Agent {
    channel: Option<String>,
    name: String,
    properties: MaybeAgentMarkerProperties,
    when: Option<mlua::Function>,
  },
  Circle {
    name: String,
    x: f32,
    y: f32,
    radius: f32,
    border_color: Option<[f32; 3]>,
    border_opacity: Option<f32>,
    border_thickness: Option<f32>,
    fill_color: Option<[f32; 3]>,
    fill_opacity: Option<f32>,
    draw_on_terrain: Option<bool>,
    draw_priority: Option<isize>,
    when: Option<mlua::Function>,
  },
  Line {
    name: String,
    points: Vec<(f32, f32)>,
    border_color: Option<[f32; 3]>,
    border_opacity: Option<f32>,
    border_thickness: Option<f32>,
    fill_color: Option<[f32; 3]>,
    fill_opacity: Option<f32>,
    draw_on_terrain: Option<bool>,
    draw_priority: Option<isize>,
    when: Option<mlua::Function>,
  },
}

impl mlua::FromLua for Marker {
  fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
    let table = match value {
      mlua::Value::Table(table) => table,
      _ => {
        return Err(mlua::Error::FromLuaConversionError {
          from: value.type_name(),
          to: "Marker".to_string(),
          message: Some("Table expected".to_string()),
        });
      }
    };

    let marker_type = table.get::<String>("type")?;
    let name = table.get::<String>("name")?;
    let when = table.get::<Option<mlua::Function>>("when")?;

    match marker_type.as_str() {
      "agent" => {
        let channel = table.get::<Option<String>>("channel")?;
        let properties = table.get::<MaybeAgentMarkerProperties>("properties")?;

        let marker = Marker::Agent {
          channel,
          name,
          properties,
          when,
        };

        Ok(marker)
      }
      "circle" => {
        let x = table.get::<f32>("x")?;
        let y = table.get::<f32>("y")?;
        let radius = table.get::<f32>("radius")?;
        let border_color = table.get::<Option<u32>>("border_color")?.map(hex_to_color);
        let border_opacity = table.get::<Option<f32>>("border_opacity")?;
        let border_thickness = table.get::<Option<f32>>("border_thickness")?;
        let fill_color = table.get::<Option<u32>>("fill_color")?.map(hex_to_color);
        let fill_opacity = table.get::<Option<f32>>("fill_opacity")?;
        let draw_on_terrain = table.get::<Option<bool>>("draw_on_terrain")?;
        let draw_priority = table.get::<Option<isize>>("draw_priority")?;

        let marker = Marker::Circle {
          name,
          x,
          y,
          radius,
          border_color,
          border_opacity,
          border_thickness,
          fill_color,
          fill_opacity,
          draw_on_terrain,
          draw_priority,
          when,
        };

        Ok(marker)
      }
      "line" => {
        let points = table
          .get::<Vec<mlua::Table>>("points")?
          .into_iter()
          .map(|table| -> mlua::Result<_> { Ok((table.get("x")?, table.get("y")?)) })
          .collect::<mlua::Result<Vec<_>>>()?;
        let border_color = table.get::<Option<u32>>("border_color")?.map(hex_to_color);
        let border_opacity = table.get::<Option<f32>>("border_opacity")?;
        let border_thickness = table.get::<Option<f32>>("border_thickness")?;
        let fill_color = table.get::<Option<u32>>("fill_color")?.map(hex_to_color);
        let fill_opacity = table.get::<Option<f32>>("fill_opacity")?;
        let draw_on_terrain = table.get::<Option<bool>>("draw_on_terrain")?;
        let draw_priority = table.get::<Option<isize>>("draw_priority")?;

        let marker = Marker::Line {
          name,
          points,
          border_color,
          border_opacity,
          border_thickness,
          fill_color,
          fill_opacity,
          draw_on_terrain,
          draw_priority,
          when,
        };

        Ok(marker)
      }
      _ => Err(mlua::Error::FromLuaConversionError {
        from: "Table",
        to: "Marker".to_string(),
        message: Some("Type expected to be one of 'agent', 'circle', 'line'".to_string()),
      }),
    }
  }
}

impl Marker {
  pub fn lua_format(&self) -> String {
    match self {
      Marker::Agent {
        channel,
        name,
        properties,
        when,
      } => format!(
        indoc!(
          r#"
          {{
            type = "agent",
            channel = {channel},
            name = {name},
            properties = {properties},
            when = {when},
          }}
        "#
        ),
        channel = channel
          .as_ref()
          .map_or("nil".to_string(), |channel| format!(r#""{}""#, channel)),
        name = format!(r#""{}""#, name),
        properties = properties.lua_format(),
        when = when
          .as_ref()
          .and_then(|when| when.info().source)
          .unwrap_or("nil".to_string()),
      ),
      Marker::Circle {
        name,
        x,
        y,
        radius,
        border_color,
        border_opacity,
        border_thickness,
        fill_color,
        fill_opacity,
        draw_on_terrain,
        draw_priority,
        when,
      } => format!(
        indoc!(
          r#"
          {{
            type = "circle",
            name = {name},
            x = {x},
            y = {y},
            radius = {radius},
            border_color = {border_color},
            border_opacity = {border_opacity},
            border_thickness = {border_thickness},
            fill_color = {fill_color},
            fill_opacity = {fill_opacity},
            draw_on_terrain = {draw_on_terrain},
            draw_priority = {draw_priority},
            when = {when},
          }}
        "#
        ),
        name = format!(r#""{}""#, name),
        x = x.to_string(),
        y = y.to_string(),
        radius = radius.to_string(),
        border_color = border_color.map_or("nil".to_string(), color_to_hex),
        border_opacity = border_opacity.map_or("nil".to_string(), |opacity| opacity.to_string()),
        border_thickness = border_thickness.map_or("nil".to_string(), |thickness| thickness.to_string()),
        fill_color = fill_color.map_or("nil".to_string(), color_to_hex),
        fill_opacity = fill_opacity.map_or("nil".to_string(), |opacity| opacity.to_string()),
        draw_on_terrain =
          draw_on_terrain.map_or("nil", |draw_on_terrain| if draw_on_terrain { "true" } else { "false" }),
        draw_priority = draw_priority.map_or("nil".to_string(), |priority| priority.to_string()),
        when = when
          .as_ref()
          .and_then(|when| when.info().source)
          .unwrap_or("nil".to_string()),
      ),
      Marker::Line {
        name,
        points,
        border_color,
        border_opacity,
        border_thickness,
        fill_color,
        fill_opacity,
        draw_on_terrain,
        draw_priority,
        when,
      } => format!(
        indoc!(
          r#"
          {{
            type = "line",
            name = {name},
            points = {points},
            border_color = {border_color},
            border_opacity = {border_opacity},
            border_thickness = {border_thickness},
            fill_color = {fill_color},
            fill_opacity = {fill_opacity},
            draw_on_terrain = {draw_on_terrain},
            draw_priority = {draw_priority},
            when = {when},
          }}
        "#
        ),
        name = format!(r#""{}""#, name),
        points = format!(
          "{{\n{}\n}}",
          points
            .iter()
            .map(|(x, y)| format!("{{ x = {}, y = {} }},", x, y))
            .collect::<Vec<_>>()
            .join("\n"),
        ),
        border_color = border_color.map_or("nil".to_string(), color_to_hex),
        border_opacity = border_opacity.map_or("nil".to_string(), |opacity| opacity.to_string()),
        border_thickness = border_thickness.map_or("nil".to_string(), |thickness| thickness.to_string()),
        fill_color = fill_color.map_or("nil".to_string(), color_to_hex),
        fill_opacity = fill_opacity.map_or("nil".to_string(), |opacity| opacity.to_string()),
        draw_on_terrain =
          draw_on_terrain.map_or("nil", |draw_on_terrain| if draw_on_terrain { "true" } else { "false" }),
        draw_priority = draw_priority.map_or("nil".to_string(), |priority| priority.to_string()),
        when = when
          .as_ref()
          .and_then(|when| when.info().source)
          .unwrap_or("nil".to_string()),
      ),
    }
  }
}
