use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use arc_swap::ArcSwap;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::ui::scripts::models::{Script, ScriptGroup, Trigger};

#[derive(Debug)]
pub enum UiMessage {
  OpenCodeEditor {
    path: PathBuf,
  },
  OpenFileExplorer {
    path: PathBuf,
  },
  AddScript {
    parent_path: PathBuf,
    name: String,
  },
  AddScriptGroup {
    parent_path: PathBuf,
    name: String,
  },
  ModifyScript {
    path: PathBuf,
    new_path: Option<PathBuf>,
    enabled: Option<bool>,
    name: Option<String>,
    throttle: Option<Option<u32>>,
    triggers: Option<Vec<Trigger>>,
    // TODO: use this for in-game code editing
    // run: Option<String>,
  },
  ModifyScriptLastTriggered {
    path: PathBuf,
    last_triggered: Option<std::time::Instant>,
  },
  ModifyScriptGroup {
    path: PathBuf,
    new_path: Option<PathBuf>,
    enabled: Option<bool>,
    name: Option<String>,
    // TODO: use this for in-game code editing
    // when: Option<String>,
  },
  RemoveScript {
    path: PathBuf,
  },
  RemoveScriptGroup {
    path: PathBuf,
  },
}

fn find_descendant_group<'a>(root_script: &'a mut ScriptGroup, path: &Path) -> Option<&'a mut ScriptGroup> {
  if !path.starts_with(&root_script.path) {
    return None;
  }

  if path == root_script.path {
    return Some(root_script);
  }

  for group in root_script.child_groups.iter_mut() {
    if !path.starts_with(&group.path) {
      continue;
    }

    if path == group.path {
      return Some(group);
    }

    return find_descendant_group(group, path);
  }

  None
}

fn find_descendant_script<'a>(root_script: &'a mut ScriptGroup, path: &Path) -> Option<&'a mut Script> {
  if !path.starts_with(&root_script.path) {
    return None;
  }

  for script in root_script.child_scripts.iter_mut() {
    if script.path == path {
      return Some(script);
    }
  }

  for group in root_script.child_groups.iter_mut() {
    if path.starts_with(&group.path) {
      return find_descendant_script(group, path);
    }
  }

  None
}

pub async fn watch_ui(
  root_script: Arc<ArcSwap<ScriptGroup>>,
  lua: Arc<mlua::Lua>,
  mut receiver: UnboundedReceiver<Vec<UiMessage>>,
) {
  log::debug!("Watching for ui changes.");

  while let Some(messages) = receiver.recv().await {
    log::debug!("UI message event received.");
    if messages.is_empty() {
      log::debug!("UI message event has no messages.");
      continue;
    }

    let lua = lua.clone();
    root_script.rcu(move |script| {
      let mut root = ScriptGroup::clone(script);

      for message in &messages {
        log::debug!("UI update message: {:?}.", message);
        match message {
          UiMessage::OpenCodeEditor { path } => {
            log::debug!("OpenCodeEditor message received.");
            crate::utils::open_in_vscode(&root.path, path);
          }
          UiMessage::OpenFileExplorer { path } => {
            log::debug!("OpenFileExplorer message received.");
            if path.is_dir() {
              crate::utils::open_in_file_explorer(path);
            } else {
              if let Some(parent) = path.parent() {
                crate::utils::open_in_file_explorer(parent);
              } else {
                log::error!(
                  "Could not open file explorer for script '{:?}' because the parent directory could not be found.",
                  path
                );
              }
            }
          }
          UiMessage::AddScript { parent_path, name } => {
            log::debug!("AddScript message received.");
            if let Some(parent) = find_descendant_group(&mut root, parent_path) {
              let path = parent_path.join(name.to_owned() + ".lua");
              let script = Script::new(lua.clone(), path.clone());
              if let Err(err) = script.update_on_disk() {
                log::error!("Could not save script '{:?}' to disk: {}.", path, err);
              }
              parent.child_scripts.push(script);
              parent.child_groups.sort_by(|a, b| a.name.cmp(&b.name));
            } else {
              log::error!(
                "Could not add script '{:?}', no parent found with path '{:?}'.",
                name,
                parent_path
              );
            }
          }
          UiMessage::AddScriptGroup { parent_path, name } => {
            log::debug!("AddScriptGroup message received");
            if let Some(parent) = find_descendant_group(&mut root, parent_path) {
              let path = parent_path.join(name);
              let group = ScriptGroup::new(lua.clone(), path.clone(), false);
              if let Err(err) = group.update_on_disk() {
                log::error!("Could not save script group '{:?}' to disk: {}.", path, err);
              }
              parent.child_groups.push(group);
              parent.child_groups.sort_by(|a, b| a.name.cmp(&b.name));
            } else {
              log::error!(
                "Could not add script '{:?}', no parent found with path '{:?}'.",
                name,
                parent_path
              );
            }
          }
          UiMessage::ModifyScript {
            path,
            new_path,
            enabled,
            name,
            throttle,
            triggers,
          } => {
            log::debug!(
              "ModifyScript message received: enabled: {:?}, name: {:?}, throttle: {:?}, triggers: {:?}.",
              enabled,
              name,
              throttle,
              triggers,
            );
            let update_script = |script: &mut Script| {
              if let Some(enabled) = enabled {
                script.enabled = *enabled;
              }
              if let Some(name) = name {
                script.name = name.clone();
              }
              if let Some(throttle) = throttle {
                script.throttle = *throttle;
              }
              if let Some(triggers) = triggers {
                script.triggers = triggers.clone();
              }
            };

            if let Some(new_path) = new_path {
              let result = path.parent().and_then(|old_parent_path| {
                let old_parent = find_descendant_group(&mut root, old_parent_path)?;
                let index = old_parent.child_scripts.iter().position(|s| s.path == *path)?;
                let mut old_script = old_parent.child_scripts.remove(index);

                update_script(&mut old_script);
                if let Err(err) = old_script.update_on_disk() {
                  log::error!("Could not save script '{:?}' to disk: {}.", path, err);
                }

                let new_parent = find_descendant_group(&mut root, new_path.parent()?)?;
                let new_script = new_parent.child_scripts.push_mut(old_script);
                if new_script.move_path(new_path, true).is_err() {
                  log::error!("Could not move script from '{:?}' to '{:?}'.", path, new_path);
                }
                new_parent.child_scripts.sort_by(|a, b| a.name.cmp(&b.name));

                Some(())
              });

              if result.is_none() {
                log::error!("Could not move script from '{:?}' to '{:?}'.", path, new_path);
              }
            } else {
              if let Some(script) = find_descendant_script(&mut root, path) {
                update_script(script);
                if let Err(err) = script.update_on_disk() {
                  log::error!("Could not save script '{:?}' to disk: {}.", path, err);
                }
              } else {
                log::error!("Could not find script with path '{:?}' to update.", path);
              }
            };
          }
          UiMessage::ModifyScriptLastTriggered { path, last_triggered } => {
            log::debug!(
              "ModifyScriptLastTriggered message received: last_triggered: {:?}.",
              last_triggered
            );
            if let Some(script) = find_descendant_script(&mut root, path) {
              script.last_triggered = *last_triggered;
            } else {
              log::error!("Could not find script with path '{:?}' to update.", path);
            }
          }
          UiMessage::ModifyScriptGroup {
            path,
            new_path,
            enabled,
            name,
          } => {
            log::debug!(
              "ModifyScriptGroup message received: enabled: {:?}, name: {:?}.",
              enabled,
              name
            );
            let update_group = |group: &mut ScriptGroup| {
              if let Some(enabled) = enabled {
                group.enabled = *enabled;
              }
              if let Some(name) = name {
                group.name = name.clone();
              }
            };

            if let Some(new_path) = new_path {
              let result = path.parent().and_then(|old_parent_path| {
                let old_parent = find_descendant_group(&mut root, old_parent_path)?;
                let index = old_parent.child_groups.iter().position(|s| s.path == *path)?;
                let mut old_group = old_parent.child_groups.remove(index);

                update_group(&mut old_group);
                if let Err(err) = old_group.update_on_disk() {
                  log::error!("Could not save script group '{:?}' to disk: {}.", path, err);
                }

                let new_parent = find_descendant_group(&mut root, new_path.parent()?)?;
                let new_group = new_parent.child_groups.push_mut(old_group);
                if new_group.move_path(new_path, true).is_err() {
                  log::error!("Could not move script group from '{:?}' to '{:?}'.", path, new_path);
                }
                new_parent.child_scripts.sort_by(|a, b| a.name.cmp(&b.name));

                Some(())
              });

              if result.is_none() {
                log::error!("Could not move script group from '{:?}' to '{:?}'.", path, new_path);
              }
            } else {
              if let Some(group) = find_descendant_group(&mut root, path) {
                update_group(group);
                if let Err(err) = group.update_on_disk() {
                  log::error!("Could not save script group '{:?}' to disk: {}.", path, err);
                }
              } else {
                log::error!("Could not find script group with path '{:?}' to update.", path);
              }
            };
          }
          UiMessage::RemoveScript { path } => {
            log::debug!("RemoveScript message received.");
            let result = path.parent().and_then(|parent_path| {
              let parent = find_descendant_group(&mut root, parent_path)?;
              let index = parent.child_scripts.iter().position(|s| s.path == *path)?;
              parent.child_scripts.remove(index);

              Some(())
            });

            if result.is_none() {
              log::error!("Could not remove script '{:?}'.", path);
            }
          }
          UiMessage::RemoveScriptGroup { path } => {
            log::debug!("RemoveScriptGroup message received.");
            let result = path.parent().and_then(|parent_path| {
              let parent = find_descendant_group(&mut root, parent_path)?;
              let index = parent.child_groups.iter().position(|s| s.path == *path)?;
              parent.child_groups.remove(index);

              Some(())
            });

            if result.is_none() {
              log::error!("Could not remove script group '{:?}'.", path);
            }
          }
        }
      }

      root
    });
  }

  log::debug!("Stopped watching for ui changes.");
}
