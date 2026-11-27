use std::sync::Arc;

use anyhow::Result;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::{gw::scan::Scanner, parallel};

pub mod agent;
pub mod camera;
pub mod chat;
pub mod hook;
pub mod interact;
pub mod item;
pub mod map;
pub mod models;
pub mod packet;
pub mod player;
pub mod render;
pub mod scan;
pub mod thread;
pub mod ui;

pub struct Gw {
  pub agent: Arc<agent::Agent>,
  pub camera: camera::Camera,
  pub chat: chat::Chat,
  pub interact: interact::Interact,
  pub item: Arc<item::Item>,
  pub map: map::Map,
  pub packet: Arc<packet::Packet>,
  pub player: player::Player,
  pub thread: Arc<thread::Thread>,
  pub ui: Arc<ui::Ui>,
}

impl Drop for Gw {
  fn drop(&mut self) {
    log::debug!("Dropping Gw");
  }
}

pub fn init() -> Result<Gw> {
  let scanner = unsafe { Scanner::new(GetModuleHandleW(None)?)? };

  let (camera, thread) = parallel![|| camera::init(&scanner), || thread::init(&scanner)]?;

  let thread = Arc::new(thread);
  let player_agent_id_ptr = camera.camera_ptr; // the first field is the player agent id

  let (packet, ui) = parallel![|| packet::init(&scanner, thread.clone()), || ui::init(
    &scanner,
    thread.clone()
  ),]?;

  let packet = Arc::new(packet);
  let ui = Arc::new(ui);

  let (chat, interact, item, map) = parallel![
    || chat::init(&scanner, ui.clone()),
    || interact::init(&scanner, thread.clone(), ui.clone()),
    || item::init(&scanner, player_agent_id_ptr, thread.clone(), ui.clone()),
    || map::init(&scanner, thread.clone(), ui.clone()),
  ]?;
  let item = Arc::new(item);

  let agent = parallel![|| agent::init(&scanner, player_agent_id_ptr, thread.clone(), ui.clone(), item.clone())]?;

  let agent = Arc::new(agent);

  let player = parallel![|| player::init(&scanner, agent.clone(), thread.clone())]?;

  log::debug!("All gw modules initialized.");

  Ok(Gw {
    agent,
    camera,
    chat,
    interact,
    item,
    map,
    packet,
    player,
    thread,
    ui,
  })
}
