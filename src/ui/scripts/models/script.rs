use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use anyhow::{Result, bail};
use hudhook::imgui::TreeNodeFlags;
use strum::IntoEnumIterator;
use tokio::{sync::mpsc::UnboundedSender, task::JoinSet};

use crate::{
  gw::models::{map::Map, skill::SkillId},
  ui::{
    common::{
      KeyMode,
      lua::{self, update_lua_values},
      render::{self, ButtonColor},
    },
    scripts::{
      ScriptTask,
      models::{Trigger, TriggerDiscriminants},
      watchers::ui::UiMessage,
    },
  },
  utils::{IS_VSCODE_INSTALLED, write_file},
};

#[derive(Clone)]
pub struct Script {
  pub enabled: bool,
  pub name: String,
  pub path: PathBuf,
  pub throttle: Option<u32>, // TODO: allow throttle to once per instance?
  pub triggers: Vec<Trigger>,
  pub run: Option<mlua::Function>,
  pub when: Option<mlua::Function>,
  pub last_triggered: Option<std::time::Instant>,
  // TODO: add toggle key
  _lua: Arc<mlua::Lua>,
}

impl Script {
  const ROOT_NAME: &str = "script";
  const FIELD_NAMES: [&str; 4] = ["enabled", "name", "throttle", "triggers"];

  pub fn new(lua: Arc<mlua::Lua>, path: PathBuf) -> Self {
    let name = path
      .file_stem()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or("<unnamed>".to_string());

    if let Ok(code) = std::fs::read_to_string(&path) {
      if let Ok(env) = lua::execute_script(&lua, &path, &code) {
        if let Ok(table) = env.get::<mlua::Table>(Self::ROOT_NAME) {
          if let Ok(script) = Self::from_table(lua.clone(), &table, path.clone(), name.clone()) {
            return script;
          } else {
            log::error!("Couldn't parse '{}' definition in script: {:?}", Self::ROOT_NAME, path);
          }
        } else {
          log::error!("Couldn't find '{}' definition in script: {:?}", Self::ROOT_NAME, path);
        }
      } else {
        log::error!("Couldn't load script: {:?}", path);
      }
    }

    Script {
      enabled: false,
      name,
      path,
      throttle: None,
      triggers: vec![],
      run: None,
      when: None,
      last_triggered: None,
      _lua: lua,
    }
  }

  fn from_table(lua: Arc<mlua::Lua>, table: &mlua::Table, path: PathBuf, name: String) -> Result<Self> {
    let enabled = table.get::<Option<bool>>("enabled")?.unwrap_or(true);
    let name = table.get::<Option<String>>("name")?.unwrap_or(name);
    let throttle = table.get::<Option<u32>>("throttle")?;
    let triggers = table
      .get::<Option<Vec<mlua::Table>>>("triggers")?
      .unwrap_or(vec![])
      .iter()
      .map(Trigger::try_from)
      .collect::<Result<Vec<_>, _>>()?;
    let run = table.get::<mlua::Function>("run")?;
    let when = table.get::<Option<mlua::Function>>("when")?;

    Ok(Self {
      enabled,
      name,
      path,
      throttle,
      triggers,
      run: Some(run),
      when,
      last_triggered: None,
      _lua: lua,
    })
  }

  pub fn move_path(&mut self, new_path: &Path, update_filesystem: bool) -> Result<()> {
    if update_filesystem {
      std::fs::rename(&self.path, new_path)?;
    }

    self.path = new_path.to_path_buf();

    Ok(())
  }

  pub fn update_on_disk(&self) -> Result<()> {
    let contents = std::fs::read_to_string(&self.path)?;

    let updated_contents = update_lua_values(contents, Self::ROOT_NAME, &Self::FIELD_NAMES, |field_name| {
      match field_name {
        // TODO: omit fields that have default values
        "enabled" => Ok(if self.enabled { "true" } else { "false" }.to_owned()),
        "name" => Ok(format!(r#""{}""#, self.name)),
        "throttle" => Ok(self.throttle.map_or("nil".to_string(), |throttle| throttle.to_string())),
        "triggers" => Ok(format!(
          "{{\n{}\n}}",
          self
            .triggers
            .iter()
            .map(|trigger| trigger
              .lua_format()
              .lines()
              .map(|line| format!("  {}", line))
              .collect::<Vec<_>>()
              .join("\n"))
            .map(|trigger| format!("{},", trigger))
            .collect::<Vec<_>>()
            .join("\n")
        )),
        _ => bail!("Invalid field name '{}'.", field_name),
      }
    })?;

    write_file(&self.path, updated_contents)?;

    Ok(())
  }

  pub fn run(&self, ui_messages_sender: &UnboundedSender<Vec<UiMessage>>, set: &mut JoinSet<()>) -> Option<ScriptTask> {
    if let Some(run) = self.run.as_ref() {
      log::debug!("Script '{:?}' was triggered.", self.path);
      let ui_messages_sender = ui_messages_sender.clone();
      let task = run.call_async::<Option<bool>>(());
      let path = self.path.clone();
      let abort_handle = set.spawn(async move {
        match task.await {
          Ok(was_run) => {
            if was_run.unwrap_or(true) {
              let _ = ui_messages_sender.send(vec![UiMessage::ModifyScriptLastTriggered {
                path,
                last_triggered: Some(std::time::Instant::now()),
              }]);
            }
          }
          Err(err) => {
            log::error!("Script '{:?}' was triggered, but failed: {}.", path, err);
          }
        }
      });

      Some(ScriptTask {
        path: self.path.clone(),
        name: self.name.clone(),
        abort_handle,
        created_at: std::time::Instant::now(),
      })
    } else {
      log::error!("No `run` function found for script '{:?}'.", self.path);
      None
    }
  }

  pub fn render(
    &self,
    ui: &hudhook::imgui::Ui,
    all_group_paths: &Vec<(&Path, &str)>,
    ui_messages_sender: &UnboundedSender<Vec<UiMessage>>,
    set: &mut JoinSet<()>,
  ) -> Vec<ScriptTask> {
    let mut changes = vec![];
    let mut tasks = vec![];

    let new_path = None;
    let mut enabled = None;
    let mut name = None;
    let mut throttle = None;
    let mut triggers = None;

    let start_pos = ui.cursor_pos();

    let inner_spacing_x = unsafe { ui.style().item_inner_spacing[0] };
    let item_spacing_x = unsafe { ui.style().item_spacing[0] };
    let checkbox_width = ui.frame_height() + inner_spacing_x;
    let button_width = render::get_icon_button_width(ui);
    let x_offset = ui.content_region_avail()[0] - checkbox_width - button_width * 3. - item_spacing_x * 3.;

    if x_offset > 0. {
      ui.set_cursor_pos([start_pos[0] + x_offset, start_pos[1]]);
    }

    if render::icon_button(ui, "\u{f04b}", "Run script", ButtonColor::Green)
      && let Some(task) = self.run(ui_messages_sender, set)
    {
      tasks.push(task);
    }

    ui.same_line();

    // ---------- change group ----------
    if render::icon_button(ui, "\u{f074}", "Change group", ButtonColor::None) {
      ui.open_popup("change_group");
    }

    ui.popup("change_group", || {
      for (group_path, group_name) in all_group_paths {
        if self.path.parent() == Some(group_path) {
          continue;
        }
        if ui.selectable(group_name)
          && let Some(file_name) = self.path.file_name()
        {
          changes.push(UiMessage::ModifyScript {
            path: self.path.clone(),
            new_path: Some(group_path.join(file_name)),
            enabled: None,
            name: None,
            throttle: None,
            triggers: None,
          });
        }
      }
    });

    ui.same_line();

    // ---------- delete script ----------
    if render::icon_button(ui, "\u{f00d}", "Delete script", ButtonColor::Red) {
      changes.push(UiMessage::RemoveScript {
        path: self.path.clone(),
      });
    }

    ui.same_line();

    let mut updated_enabled = self.enabled;
    if ui.checkbox("##enabled", &mut updated_enabled) {
      enabled = Some(updated_enabled);
    }
    if ui.is_item_hovered() {
      ui.tooltip_text(if self.enabled {
        "Disable script"
      } else {
        "Enable script"
      });
    }

    ui.set_cursor_pos(start_pos);

    let tree = ui
      .tree_node_config(&self.name)
      .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
      .push();
    if ui.is_item_hovered() {
      ui.tooltip_text(self.path.to_string_lossy());
    }

    if let Some(_token) = tree {
      ui.indent();

      // ---------- name ----------
      let mut updated_name = self.name.clone();
      if ui.input_text("Script Name", &mut updated_name).build() {
        name = Some(updated_name);
      }

      // ---------- open file explorer ----------
      ui.same_line();
      if render::icon_button(ui, "\u{f07c}", "Open in file explorer", ButtonColor::None) {
        changes.push(UiMessage::OpenFileExplorer {
          path: self.path.clone(),
        });
      }

      // ---------- open vscode ----------
      if *IS_VSCODE_INSTALLED {
        ui.same_line();
        if render::icon_button(ui, "\u{f044}", "Open in code editor", ButtonColor::None) {
          changes.push(UiMessage::OpenCodeEditor {
            path: self.path.clone(),
          });
        }
      }

      // ---------- throttle ----------
      throttle = {
        let _token = ui.push_id("throttle");
        let mut throttle = self.throttle.unwrap_or(0);
        let result = if ui.input_scalar("Throttle", &mut throttle).build() {
          Some(if throttle > 0 { Some(throttle) } else { None })
        } else {
          None
        };

        if ui.is_item_hovered() {
          ui.tooltip_text("Prevent script from triggering again until this many milliseconds since the last trigger. Set to 0 to disable.");
        }

        result
      };

      // ---------- triggers ----------
      triggers = {
        let _token = ui.push_id("triggers");

        let mut triggers = None;

        ui.text("Triggers:");

        ui.same_line();

        // ---------- add new trigger ----------
        {
          if render::icon_button(ui, "\u{f067}", "Add new trigger", ButtonColor::Green) {
            ui.open_popup("add_trigger");
          }

          ui.popup("add_trigger", || {
            for trigger_type in TriggerDiscriminants::iter() {
              if ui.selectable(trigger_type.as_ref()) {
                triggers
                  .get_or_insert_with(|| self.triggers.clone())
                  .push(trigger_type.into());
              }
            }
          });
        }

        if let Some(_table) = ui.begin_table("triggers_list", 2) {
          for (index, trigger) in self.triggers.iter().enumerate() {
            let _id = ui.push_id_usize(index);

            ui.table_next_row();
            ui.table_next_column();

            match trigger {
              Trigger::Key {
                keys,
                mode,
                alt,
                ctrl,
                shift,
                win,
              } => {
                match mode {
                  KeyMode::Hold => ui.text("Every frame whilst key held"),
                  KeyMode::Press => ui.text("On key press"),
                }

                ui.same_line();

                let mut hotkey = render::Hotkey {
                  keys: keys.clone(),
                  mode: *mode,
                  alt: *alt,
                  ctrl: *ctrl,
                  shift: *shift,
                  win: *win,
                };
                if render::hotkey_box(ui, &mut hotkey) {
                  triggers.get_or_insert_with(|| self.triggers.clone())[index] = Trigger::Key {
                    keys: hotkey.keys,
                    mode: hotkey.mode,
                    alt: hotkey.alt,
                    ctrl: hotkey.ctrl,
                    shift: hotkey.shift,
                    win: hotkey.win,
                  };
                }
              }
              Trigger::Map { map_id, outpost } => {
                ui.text("On map change");

                ui.same_line();

                let mut current_map_id = map_id.map_or(0, |map| map as u32);
                if ui.input_scalar("Map ID", &mut current_map_id).build() {
                  triggers.get_or_insert_with(|| self.triggers.clone())[index] = Trigger::Map {
                    map_id: if current_map_id == 0 {
                      None
                    } else {
                      Map::from_repr(current_map_id)
                    },
                    outpost: *outpost,
                  };
                }

                ui.same_line();

                let mut current_outpost = match *outpost {
                  None => 0,
                  Some(false) => 1,
                  Some(true) => 2,
                };
                let options = vec!["Any", "Explorable", "Outpost"];
                if ui.combo("##map_outpost", &mut current_outpost, &options, |&x| x.into()) {
                  triggers.get_or_insert_with(|| self.triggers.clone())[index] = Trigger::Map {
                    map_id: if current_map_id == 0 {
                      None
                    } else {
                      Map::from_repr(current_map_id)
                    },
                    outpost: match current_outpost {
                      1 => Some(false),
                      2 => Some(true),
                      _ => None,
                    },
                  };
                }
              }
              Trigger::Skill { skill_id, finished } => {
                ui.text("On skill used");

                ui.same_line();

                let mut current_skill_id = skill_id.map_or(0, |skill| skill as u32);
                if ui.input_scalar("Skill ID", &mut current_skill_id).build() {
                  triggers.get_or_insert_with(|| self.triggers.clone())[index] = Trigger::Skill {
                    skill_id: if current_skill_id == 0 {
                      None
                    } else {
                      SkillId::from_repr(current_skill_id)
                    },
                    finished: *finished,
                  };
                }

                ui.same_line();

                let mut current_finished = if *finished { 1 } else { 0 };
                let options = vec!["Used", "Finished"];
                if ui.combo("##skill_finished", &mut current_finished, &options, |&x| x.into()) {
                  triggers.get_or_insert_with(|| self.triggers.clone())[index] = Trigger::Skill {
                    skill_id: if current_skill_id == 0 {
                      None
                    } else {
                      SkillId::from_repr(current_skill_id)
                    },
                    finished: current_finished != 0,
                  };
                }
              }
              Trigger::Tick => {
                ui.text("Every frame");
              }
            }

            ui.table_next_column();

            // ---------- remove trigger ----------
            if render::icon_button(ui, "\u{f00d}", "Delete trigger", ButtonColor::Red) {
              triggers.get_or_insert_with(|| self.triggers.clone()).remove(index);
            }
          }
        }

        triggers
      };
    }

    if new_path.is_some() || enabled.is_some() || name.is_some() || throttle.is_some() || triggers.is_some() {
      changes.push(UiMessage::ModifyScript {
        path: self.path.clone(),
        new_path,
        enabled,
        name,
        throttle,
        triggers,
      });
    }

    if !changes.is_empty() {
      let _ = ui_messages_sender.send(changes);
    }

    tasks
  }
}
