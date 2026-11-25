use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use arc_swap::ArcSwap;
use hudhook::imgui;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{Debouncer, new_debouncer};
use tokio::{
  sync::mpsc::{UnboundedReceiver, UnboundedSender},
  task::{AbortHandle, JoinSet},
};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{
  gw::Gw,
  ui::{
    Module,
    common::KeyState,
    config::Config,
    scripts::{
      models::ScriptGroup,
      watchers::{
        files::watch_files,
        triggers::watch_script_triggers,
        ui::{UiMessage, watch_ui},
      },
    },
  },
  utils::CONFIG_PATH,
};

mod models;
mod watchers;

#[derive(Clone)]
pub struct ScriptTask {
  pub name: String,
  pub path: PathBuf,
  pub abort_handle: AbortHandle,
  pub created_at: std::time::Instant,
}

pub struct Scripts {
  // drop order is important here
  _debouncer: Debouncer<RecommendedWatcher>,
  root_script: Arc<ArcSwap<ScriptGroup>>,
  tasks: Arc<ArcSwap<Vec<ScriptTask>>>,
  ui_messages_sender: UnboundedSender<Vec<UiMessage>>,
  handle: tokio::runtime::Handle,
  _cancel_guard: DropGuard,
}

impl Module for Scripts {
  fn render(&mut self, _ui: &imgui::Ui, _config: &mut Config) -> bool {
    false
  }

  fn render_settings(&mut self, ui: &imgui::Ui, _config: &mut Config) -> bool {
    let _guard = self.handle.enter();
    let mut set = JoinSet::new();

    let root_script = self.root_script.load();
    let tasks = self.tasks.load();

    let new_tasks = root_script.render(
      ui,
      true,
      &tasks,
      &root_script.get_descendant_groups(),
      &self.ui_messages_sender,
      &mut set,
    );
    if !new_tasks.is_empty() || tasks.iter().any(|task| task.abort_handle.is_finished()) {
      self.tasks.rcu(|tasks| {
        let mut valid_tasks = tasks
          .iter()
          .filter(|task| !task.abort_handle.is_finished())
          .cloned()
          .collect::<Vec<_>>();
        valid_tasks.extend(new_tasks.clone());
        Arc::new(valid_tasks)
      });
    }

    false
  }
}

pub fn init(
  gw_api: Arc<Gw>,
  lua: Arc<mlua::Lua>,
  key_state_receiver: UnboundedReceiver<KeyState>,
  handle: tokio::runtime::Handle,
  cancel_token: CancellationToken,
) -> Result<Scripts> {
  let scripts_path = CONFIG_PATH.join("scripts");
  std::fs::create_dir_all(&scripts_path)?;

  log::debug!("Annotations written.");

  let (tx, on_file_event) = tokio::sync::mpsc::unbounded_channel();
  let mut debouncer = new_debouncer(Duration::from_millis(500), move |result| match result {
    Ok(events) => {
      let _ = tx.send(events);
    }
    Err(err) => {
      log::warn!("File watcher sent an error: {:?}", err);
    }
  })?;
  debouncer.watcher().watch(&scripts_path, RecursiveMode::Recursive)?;
  log::debug!("File watcher initialized.");

  let root_script = Arc::new(ArcSwap::from_pointee(ScriptGroup::new(
    lua.clone(),
    scripts_path.clone(),
    true,
  )));
  let root_script_ref = root_script.clone();

  let (ui_messages_sender, ui_messages_receiver) = tokio::sync::mpsc::unbounded_channel();
  let ui_messages_sender_ref = ui_messages_sender.clone();

  let tasks = Arc::new(ArcSwap::from_pointee(vec![]));
  let tasks_ref = tasks.clone();

  let _cancel_guard = cancel_token.clone().drop_guard();
  tokio::spawn(async move {
    let mut set = tokio::task::JoinSet::new();

    // watch for in-game actions and trigger scripts
    set.spawn(watch_script_triggers(
      gw_api,
      key_state_receiver,
      root_script_ref.clone(),
      tasks_ref,
      ui_messages_sender_ref,
      lua.clone(),
    ));
    // watch for file system changes and reload scripts
    set.spawn(watch_files(root_script_ref.clone(), lua.clone(), on_file_event));
    // watch for messages queued by the ui
    set.spawn(watch_ui(root_script_ref, lua.clone(), ui_messages_receiver));

    cancel_token.cancelled().await;
    set.shutdown().await;
  });

  Ok(Scripts {
    _debouncer: debouncer,
    root_script,
    tasks,
    ui_messages_sender,
    handle,
    _cancel_guard,
  })
}
