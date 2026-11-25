use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use anyhow::{Result, bail};
use hudhook::imgui::{self, TreeNodeFlags};
use indoc::indoc;
use tokio::{sync::mpsc::UnboundedSender, task::JoinSet};

use crate::{
  ui::{
    common::{
      lua::{self, update_lua_values},
      render::{self, ButtonColor},
    },
    scripts::{ScriptTask, models::Script, watchers::ui::UiMessage},
  },
  utils::write_file,
};

#[derive(Clone)]
pub struct ScriptGroup {
  pub enabled: bool,
  pub name: String,
  pub path: PathBuf,
  pub when: Option<mlua::Function>,
  pub child_groups: Vec<ScriptGroup>,
  pub child_scripts: Vec<Script>,
  // TODO: add toggle key
  _lua: Arc<mlua::Lua>,
}

impl ScriptGroup {
  const ROOT_NAME: &str = "group";
  const FIELD_NAMES: [&str; 2] = ["enabled", "name"];

  pub fn new(lua: Arc<mlua::Lua>, path: PathBuf, is_root: bool) -> Self {
    let mut enabled = true;
    let mut child_groups = vec![];
    let mut child_scripts = vec![];

    if let Ok(entries) = std::fs::read_dir(&path) {
      for entry in entries {
        if let Ok(entry) = entry
          && let Ok(file_type) = entry.file_type()
        {
          if file_type.is_dir() {
            child_groups.push(ScriptGroup::new(lua.clone(), entry.path(), false));
          } else if entry.file_name().to_string_lossy().ends_with(".lua") {
            if entry.file_name() == "_meta.lua" || entry.file_name() == "_.lua" {
              continue;
            }
            child_scripts.push(Script::new(lua.clone(), entry.path()));
          }
        }
      }
    } else {
      enabled = false;
    }

    child_groups.sort_by(|a, b| a.name.cmp(&b.name));
    child_scripts.sort_by(|a, b| a.name.cmp(&b.name));

    let name = if is_root {
      "Scripts".to_string()
    } else {
      path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or("<unnamed>".to_string())
    };

    let group_file_path = path.join("_.lua");

    if let Ok(code) = std::fs::read_to_string(&group_file_path) {
      if let Ok(env) = lua::execute_script(&lua, &group_file_path, &code) {
        if let Ok(table) = env.get::<mlua::Table>(Self::ROOT_NAME) {
          if let Ok(mut script_group) = Self::from_table(lua.clone(), &table, path.clone(), name.clone()) {
            script_group.child_groups = child_groups;
            script_group.child_scripts = child_scripts;
            return script_group;
          } else {
            log::error!(
              "Couldn't parse '{}' definition in script group: {:?}",
              Self::ROOT_NAME,
              &path
            );
          }
        } else {
          log::error!(
            "Couldn't find '{}' definition in script group: {:?}",
            Self::ROOT_NAME,
            &path
          );
        }
      } else {
        log::error!("Couldn't load script group: {:?}", &path);
      }
    }

    Self {
      enabled,
      name,
      path,
      when: None,
      child_groups,
      child_scripts,
      _lua: lua,
    }
  }

  fn from_table(lua: Arc<mlua::Lua>, table: &mlua::Table, path: PathBuf, name: String) -> Result<Self> {
    let enabled = table.get::<Option<bool>>("enabled")?.unwrap_or(true);
    let name = table.get::<Option<String>>("name")?.unwrap_or(name);
    let when = table.get::<Option<mlua::Function>>("when")?;

    Ok(Self {
      enabled,
      name,
      path,
      when,
      child_groups: vec![],
      child_scripts: vec![],
      _lua: lua,
    })
  }

  pub fn get_descendant_groups(&self) -> Vec<(&Path, &str)> {
    let mut result = vec![(self.path.as_path(), self.name.as_str())];
    result.extend(self.child_groups.iter().flat_map(|group| group.get_descendant_groups()));
    result
  }

  pub fn move_path(&mut self, new_path: &Path, update_filesystem: bool) -> Result<()> {
    if update_filesystem {
      std::fs::rename(&self.path, new_path)?;
    }

    for group in self.child_groups.iter_mut() {
      if let Some(name) = group.path.file_name() {
        group.move_path(&new_path.join(name), false)?;
      }
    }

    for script in self.child_scripts.iter_mut() {
      if let Some(name) = script.path.file_name() {
        script.move_path(&new_path.join(name), false)?;
      }
    }

    self.path = new_path.to_path_buf();

    Ok(())
  }

  pub fn update_on_disk(&self) -> Result<()> {
    std::fs::create_dir_all(&self.path)?;
    let file_path = self.path.join("_.lua");

    let updated_contents = if let Ok(contents) = std::fs::read_to_string(&file_path) {
      Some(update_lua_values(
        contents,
        Self::ROOT_NAME,
        &Self::FIELD_NAMES,
        |field_name| {
          // TODO: omit fields that have default values
          match field_name {
            "enabled" => Ok(if self.enabled { "true" } else { "false" }.to_owned()),
            "name" => Ok(format!(r#""{}""#, self.name)),
            _ => bail!("Invalid field name '{}'.", field_name),
          }
        },
      )?)
    } else {
      let mut fields = vec![];
      if self
        .path
        .file_name()
        .map(|name| name.to_string_lossy() != self.name)
        .unwrap_or(true)
      {
        fields.push(format!(r#"  name = "{}","#, self.name));
      }
      if !self.enabled {
        fields.push("  enabled = false,".to_string());
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
      write_file(&file_path, updated_contents)?;
    }

    Ok(())
  }

  pub fn render(
    &self,
    ui: &imgui::Ui,
    is_root: bool,
    _tasks: &[ScriptTask],
    all_group_paths: &Vec<(&Path, &str)>,
    ui_messages_sender: &UnboundedSender<Vec<UiMessage>>,
    set: &mut JoinSet<()>,
  ) -> Vec<ScriptTask> {
    let mut changes = vec![];
    let mut tasks = vec![];

    let new_path = None; // TODO: allow changing this?
    let mut enabled = None;
    let mut name = None;

    let start_pos = ui.cursor_pos();

    let item_spacing_x = unsafe { ui.style().item_spacing[0] };
    let checkbox_width = render::get_checkbox_width(ui);
    let button_width = render::get_icon_button_width(ui);
    let x_offset = ui.content_region_avail()[0] - checkbox_width - button_width * 3. - item_spacing_x * 3.;

    if x_offset > 0. {
      ui.set_cursor_pos([start_pos[0] + x_offset, start_pos[1]]);
    }

    // ---------- add new group/script ----------
    if render::icon_button(ui, "\u{f067}", "Add new group/script", ButtonColor::Green) {
      ui.open_popup("add_group_or_script");
    }

    ui.popup("add_group_or_script", || {
      if ui.selectable("Group") {
        changes.push(UiMessage::AddScriptGroup {
          parent_path: self.path.clone(),
          name: "New script".to_string(),
        });
      }
      if ui.selectable("Script") {
        changes.push(UiMessage::AddScript {
          parent_path: self.path.clone(),
          name: "New script".to_string(),
        });
      }
    });

    ui.same_line();

    // ---------- change group ----------
    if render::icon_button(ui, "\u{f074}", "Change group", ButtonColor::None) {
      ui.open_popup("change_group");
    }

    ui.popup("change_group", || {
      for (group_path, group_name) in all_group_paths {
        if self.path == *group_path {
          continue;
        }
        if ui.selectable(group_name)
          && let Some(file_name) = self.path.file_name()
        {
          changes.push(UiMessage::ModifyScriptGroup {
            path: self.path.clone(),
            new_path: Some(group_path.join(file_name)),
            enabled: None,
            name: None,
          });
        }
      }
    });

    ui.same_line();

    // ---------- delete group ----------
    if render::icon_button(ui, "\u{f00d}", "Delete group and all children", ButtonColor::Red) {
      changes.push(UiMessage::RemoveScriptGroup {
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
        "Disable script group"
      } else {
        "Enable script group"
      });
    }

    ui.set_cursor_pos(start_pos);

    let tree = {
      let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
      let tree = ui
        .tree_node_config(&self.name)
        .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
        .push();
      if ui.is_item_hovered() {
        ui.tooltip_text(self.path.to_string_lossy());
      }

      tree
    };

    if let Some(_token) = tree {
      ui.indent();

      if !is_root {
        // ---------- name ----------
        if let Some(_table) = ui.begin_table("name", 2) {
          ui.table_next_row();
          ui.table_next_column();
          ui.text("Group Name:");

          ui.table_next_column();
          ui.set_next_item_width(-1.0);
          let mut updated_name = self.name.clone();
          if ui.input_text("##name", &mut updated_name).build() {
            name = Some(updated_name);
          }
        }
      }

      // ---------- child scripts ----------
      {
        let _token = ui.push_id("scripts");
        for (index, script) in self.child_scripts.iter().enumerate() {
          let _id = ui.push_id_usize(index);
          tasks.extend(script.render(ui, all_group_paths, ui_messages_sender, set));
        }
      }

      ui.spacing();
      ui.group(|| ui.separator());
      ui.spacing();

      // ---------- child groups ----------
      {
        let _token = ui.push_id("groups");
        for (index, group) in self.child_groups.iter().enumerate() {
          let _id = ui.push_id_usize(index);
          tasks.extend(group.render(ui, false, &[], all_group_paths, ui_messages_sender, set));
        }
      }

      // ---------- task manager ----------
      if is_root && !tasks.is_empty() {
        let _token = ui.push_id("tasks");
        ui.text(format!("Running scripts: {}", tasks.len()));

        let headers = [
          imgui::TableColumnSetup::new("Script"),
          imgui::TableColumnSetup::new("Elapsed"),
          imgui::TableColumnSetup::new("Abort"),
        ];
        if let Some(_token) = ui.begin_table_header("tasks", headers) {
          for (index, task) in tasks.iter().enumerate() {
            let _id = ui.push_id_usize(index);

            ui.table_next_row();

            ui.table_next_column();

            ui.text(&task.name);
            if ui.is_item_hovered() {
              ui.tooltip_text(task.path.to_string_lossy());
            }

            ui.table_next_column();

            ui.text(format!("{:.1}s", task.created_at.elapsed().as_secs_f32()));

            ui.table_next_column();

            if ui.button("Abort") {
              task.abort_handle.abort();
            }
          }
        }
      }

      ui.unindent();
    }

    if enabled.is_some() || new_path.is_some() || enabled.is_some() || name.is_some() {
      changes.push(UiMessage::ModifyScriptGroup {
        path: self.path.clone(),
        new_path,
        enabled,
        name,
      });
    }

    if !changes.is_empty() {
      let _ = ui_messages_sender.send(changes);
    }

    tasks
  }
}
