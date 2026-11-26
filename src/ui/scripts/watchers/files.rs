use std::sync::Arc;

use arc_swap::ArcSwap;
use notify_debouncer_mini::DebouncedEventBatch;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::ui::scripts::models::ScriptGroup;

pub async fn watch_files(
  root_script: Arc<ArcSwap<ScriptGroup>>,
  lua: Arc<mlua::Lua>,
  mut on_file_events: UnboundedReceiver<DebouncedEventBatch>,
) {
  log::debug!("watching for file changes");

  let mut events = vec![];

  while on_file_events.recv_many(&mut events, 20).await > 0 {
    if !events.is_empty() {
      // TODO: only update the necessary scripts to improve performance
      log::info!("Root script modified; reloading.");

      let updated_root_script = ScriptGroup::new(lua.clone(), root_script.load().path.to_path_buf(), true);

      root_script.store(Arc::new(updated_root_script));
    }
  }

  log::debug!("stopped watching for file changes");
}
