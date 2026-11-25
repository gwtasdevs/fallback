use anyhow::{Error, Result, bail};
use indoc::indoc;
use strum::{AsRefStr, EnumDiscriminants, EnumIter, EnumString};

use crate::{
  gw::models,
  ui::common::{Key, KeyMode},
};

#[derive(Clone, Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumIter, EnumString))]
pub enum Trigger {
  Key {
    keys: Vec<Key>,
    mode: KeyMode,
    alt: bool,
    ctrl: bool,
    shift: bool,
    win: bool,
  },
  Map {
    map_id: Option<models::map::Map>,
    outpost: Option<bool>,
  },
  Skill {
    skill_id: Option<models::skill::SkillId>,
    finished: bool,
  },
  Tick,
}

impl Trigger {
  pub fn lua_format(&self) -> String {
    match self {
      Self::Key {
        keys,
        mode,
        alt,
        ctrl,
        shift,
        win,
      } => format!(
        indoc!(
          r#"
          {{
            type = "key",
            keys = {{{keys}}},
            mode = {mode},
            alt = {alt},
            ctrl = {ctrl},
            shift = {shift},
            win = {win},
          }}
        "#
        ),
        keys = keys
          .iter()
          .map(|key| format!("Key.{}", Into::<&str>::into(key)))
          .collect::<Vec<_>>()
          .join(", "),
        mode = format!("KeyMode.{}", Into::<&str>::into(mode)),
        alt = if *alt { "true" } else { "false" },
        ctrl = if *ctrl { "true" } else { "false" },
        shift = if *shift { "true" } else { "false" },
        win = if *win { "true" } else { "false" },
      ),
      Self::Map { map_id, outpost } => format!(
        r#"{{ type = "map", map_id = {map_id}, outpost = {outpost} }}"#,
        map_id = map_id.map_or("nil".to_owned(), |map| format!("Map.{}", Into::<&str>::into(map))),
        outpost = outpost.map_or("nil", |is_outpost| if is_outpost { "true" } else { "false" }),
      ),
      Self::Skill { skill_id, finished } => format!(
        r#"{{ type = "skill", skill_id = {skill_id}, finished = {finished} }}"#,
        skill_id = skill_id.map_or("nil".to_owned(), |skill| format!("Skill.{}", Into::<&str>::into(skill))),
        finished = if *finished { "true" } else { "false" },
      ),
      Self::Tick => r#"{ type = "tick" }"#.to_string(),
    }
    .trim()
    .to_string()
  }
}

impl TryFrom<&mlua::Table> for Trigger {
  type Error = Error;

  fn try_from(table: &mlua::Table) -> Result<Self> {
    let trigger_type = table.get::<String>("type")?;
    match trigger_type.as_str() {
      "key" => Ok(Trigger::Key {
        keys: table.get::<Vec<Key>>("keys")?,
        mode: table.get::<Option<KeyMode>>("mode")?.unwrap_or(KeyMode::default()),
        alt: table.get::<Option<bool>>("alt")?.unwrap_or(false),
        ctrl: table.get::<Option<bool>>("ctrl")?.unwrap_or(false),
        shift: table.get::<Option<bool>>("shift")?.unwrap_or(false),
        win: table.get::<Option<bool>>("win")?.unwrap_or(false),
      }),
      "map" => Ok(Trigger::Map {
        map_id: table.get::<Option<models::map::Map>>("map_id")?,
        outpost: table.get::<Option<bool>>("outpost")?,
      }),
      "skill" => Ok(Trigger::Skill {
        skill_id: table.get::<Option<models::skill::SkillId>>("skill_id")?,
        finished: table.get::<Option<bool>>("finished")?.unwrap_or(true),
      }),
      "tick" => Ok(Trigger::Tick),
      _ => bail!("Invalid trigger type '{}'.", trigger_type),
    }
  }
}

impl From<TriggerDiscriminants> for Trigger {
  fn from(discriminant: TriggerDiscriminants) -> Self {
    match discriminant {
      TriggerDiscriminants::Key => Trigger::Key {
        keys: vec![],
        mode: KeyMode::default(),
        alt: false,
        ctrl: false,
        shift: false,
        win: false,
      },
      TriggerDiscriminants::Map => Trigger::Map {
        map_id: None,
        outpost: None,
      },
      TriggerDiscriminants::Skill => Trigger::Skill {
        skill_id: None,
        finished: false,
      },
      TriggerDiscriminants::Tick => Trigger::Tick,
    }
  }
}
