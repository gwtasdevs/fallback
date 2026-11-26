use std::sync::Arc;

use arc_swap::ArcSwap;
use notify_debouncer_mini::DebouncedEventBatch;
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::models::marker_group::MarkerGroup;

pub async fn watch_files(
  markers_root: Arc<ArcSwap<MarkerGroup>>,
  lua: Arc<mlua::Lua>,
  mut on_file_events: UnboundedReceiver<DebouncedEventBatch>,
) {
  log::debug!("watching for file changes");

  let mut events = vec![];

  while on_file_events.recv_many(&mut events, 20).await > 0 {
    if !events.is_empty() {
      // TODO: only update the necessary groups to improve performance
      log::info!("Root marker group modified; reloading.");

      let updated_root = MarkerGroup::new(lua.clone(), markers_root.load().path.clone());

      markers_root.store(Arc::new(updated_root));
    }
  }

  log::debug!("stopped watching for file changes");
}
