use std::sync::Arc;

use anyhow::{Context, Result};

use crate::{
  gw::{
    models::{
      self, Array,
      ui_message::{SkillActivatedUiMessage, SkillFinishedUiMessage},
    },
    scan::Scanner,
  },
  parallel, pattern, read_ptr,
};

pub struct Agent {
  thread: Arc<crate::gw::thread::Thread>,
  ui: Arc<crate::gw::ui::Ui>,
  item: Arc<crate::gw::item::Item>,
  player_agent_id_ptr: usize,

  agent_array: usize,
  change_target_fn: unsafe extern "cdecl" fn(agent_id: usize, usize) -> (),
  target_agent_id_ptr: usize,
  pub agent_skill_cast_ui_message_id: usize,
  pub skill_finished_ui_message_id: usize,
}

impl Drop for Agent {
  fn drop(&mut self) {
    log::debug!("Dropping Agent");
  }
}

impl Agent {
  pub fn agents(&self) -> Array<*const models::agent::BaseAgent> {
    unsafe { Array::<*const models::agent::BaseAgent>::from_address(self.agent_array) }
  }

  pub fn get_base_agent<T>(&self, agent_id: T) -> Option<*const models::agent::BaseAgent>
  where
    T: TryInto<usize>,
  {
    let agent_id: usize = agent_id.try_into().ok()?;
    if agent_id == 0 {
      return None;
    }

    self
      .agents()
      .get(agent_id)
      .filter(|base_agent_ptr| !base_agent_ptr.is_null())
  }

  pub fn get_agent<T>(&self, agent_id: T) -> Option<*const models::agent::Agent>
  where
    T: TryInto<usize>,
  {
    if let Some(base) = self.get_base_agent(agent_id) {
      models::agent::into_agent(base).ok()
    } else {
      None
    }
  }

  pub fn get_gadget<T>(&self, agent_id: T) -> Option<*const models::agent::Gadget>
  where
    T: TryInto<usize>,
  {
    if let Some(base) = self.get_base_agent(agent_id) {
      models::agent::into_gadget(base).ok()
    } else {
      None
    }
  }

  pub fn get_item<T>(&self, agent_id: T) -> Option<*const models::agent::Item>
  where
    T: TryInto<usize>,
  {
    if let Some(base) = self.get_base_agent(agent_id) {
      models::agent::into_item(base).ok()
    } else {
      None
    }
  }

  pub async fn get_agent_name(&self, agent_id: u32) -> Option<String> {
    let encoded_name = self.get_agent_encoded_name(agent_id as usize)?;

    if let Some(encoded_name) = self.ui.try_clone_encoded_string(encoded_name) {
      self.ui.decode_string(encoded_name).await
    } else {
      None
    }
  }

  pub fn try_get_agent_name(&self, agent_id: u32) -> Option<String> {
    self
      .ui
      .try_get_decoded_string(self.get_agent_encoded_name(agent_id as usize)?)
  }

  pub fn get_agent_encoded_name(&self, agent_id: usize) -> Option<*const u16> {
    if let Some(base) = self.get_base_agent(agent_id) {
      if let Ok(agent) = models::agent::into_agent(base) {
        let login_number = unsafe { (&raw const (*agent).login_number).read_unaligned() as usize };
        if login_number != 0 {
          let encoded_name = read_ptr!(self.thread.get_game_context(), .world.players[login_number].encoded_name);
          if let Some(encoded_name) = encoded_name
            && !encoded_name.is_null()
          {
            return Some(encoded_name);
          }
        }

        let encoded_name = read_ptr!(self.thread.get_game_context(), .world.agent_infos[agent_id].encoded_name);
        if let Some(encoded_name) = encoded_name
          && !encoded_name.is_null()
        {
          return Some(encoded_name);
        }

        let model_id = unsafe { (&raw const (*agent).model_id).read_unaligned() as usize };
        let encoded_name = read_ptr!(self.thread.get_game_context(), .world.npcs[model_id].encoded_name);
        if let Some(encoded_name) = encoded_name
          && !encoded_name.is_null()
        {
          return Some(encoded_name);
        }
      } else if let Ok(gadget) = models::agent::into_gadget(base) {
        if let Some(summary_info) = read_ptr!(self.thread.get_game_context(), .agent.agent_summaries[agent_id].info) {
          let encoded_name = read_ptr!(summary_info, .encoded_name);
          if let Some(encoded_name) = encoded_name
            && !encoded_name.is_null()
          {
            return Some(encoded_name);
          }

          if let Some(gadget_id) = read_ptr!(gadget, .gadget_id).or(read_ptr!(summary_info, .gadget_id)) {
            let encoded_name = read_ptr!(self.thread.get_game_context(), .gadget.gadget_info[gadget_id].encoded_name);
            if let Some(encoded_name) = encoded_name
              && !encoded_name.is_null()
            {
              return Some(encoded_name);
            }
          }
        }
      } else if let Ok(item) = models::agent::into_item(base) {
        let item_id = unsafe { (&raw const (*item).item_id).read_unaligned() };
        if let Some(item) = self.item.get_item_by_id(item_id) {
          let encoded_name = unsafe { (&raw const (*item).encoded_name).read_unaligned() };
          if !encoded_name.is_null() {
            return Some(encoded_name);
          }
        }
      }
    }

    let encoded_name = read_ptr!(self.thread.get_game_context(), .world.agent_infos[agent_id].encoded_name);
    if let Some(encoded_name) = encoded_name
      && !encoded_name.is_null()
    {
      Some(encoded_name)
    } else {
      None
    }
  }

  pub fn get_player(&self) -> Option<*const models::agent::Agent> {
    self.get_agent(self.get_player_agent_id())
  }

  pub fn get_player_agent_id(&self) -> usize {
    unsafe { (self.player_agent_id_ptr as *const usize).read_unaligned() }
  }

  pub fn get_target(&self) -> Option<*const models::agent::BaseAgent> {
    self.get_base_agent(self.get_target_agent_id())
  }

  pub fn get_target_agent_id(&self) -> usize {
    unsafe { (self.target_agent_id_ptr as *const usize).read_unaligned() }
  }

  pub fn change_target(&self, agent_id: usize) {
    let func = self.change_target_fn;

    self.thread.fire_and_forget(move || {
      unsafe { func(agent_id, 0) };
    });
  }

  pub fn register_skill_activated_hook<F>(&self, func: F) -> uuid::Uuid
  where
    F: Fn(*const SkillActivatedUiMessage) + Send + Sync + 'static,
  {
    self
      .ui
      .register_send_ui_message_hook(self.agent_skill_cast_ui_message_id, move |message| {
        func(message);
      })
  }

  pub fn register_skill_finished_hook<F>(&self, func: F) -> uuid::Uuid
  where
    F: Fn(*const SkillFinishedUiMessage) + Send + Sync + 'static,
  {
    self
      .ui
      .register_send_ui_message_hook(self.skill_finished_ui_message_id, move |message| {
        func(message);
      })
  }
}

pub fn init(
  scanner: &Scanner,
  player_agent_id_ptr: usize,
  thread: Arc<crate::gw::thread::Thread>,
  ui: Arc<crate::gw::ui::Ui>,
  item: Arc<crate::gw::item::Item>,
) -> Result<Agent> {
  let (
    agent_array,
    (change_target_fn, target_agent_id_ptr),
    agent_skill_cast_ui_message_id,
    skill_finished_ui_message_id,
  ) = parallel![
    || find_agent_array_pointer(scanner).context("Could not find agent_array pointer."),
    || find_change_target_function(scanner).context("Could not find change_target function."),
    || find_agent_skill_cast_ui_message_id(scanner, ui.send_ui_message_fn)
      .context("Could not find agent_skill_cast ui message id."),
    || find_skill_finished_ui_message_id(scanner, ui.send_ui_message_fn)
      .context("Could not find skill_finished ui message id."),
  ]?;

  Ok(Agent {
    thread,
    ui,
    item,
    player_agent_id_ptr,

    agent_array,
    change_target_fn: unsafe { std::mem::transmute(change_target_fn) },
    target_agent_id_ptr,
    agent_skill_cast_ui_message_id,
    skill_finished_ui_message_id,
  })
}

fn find_agent_array_pointer(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_wide_string("Sequence queue congestion")?;
  let bytes = address.to_le_bytes();

  let pattern = pattern![
    0x68, bytes[0], bytes[1], bytes[2], bytes[3], // PUSH address
    0x6a, ?,                                      // PUSH ...
    0xe8, ?, ?, ?, ?,                             // CALL ...
    0x83, 0xc4, ?,                                // ADD ESP, ...
    0xe8, ?, ?, ?, ?,                             // CALL ...
    0x8b, ?,                                      // MOV ..., [agent_array]
  ];
  let address = scanner.scan(&pattern)? + pattern.len();
  let agent_array = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned agent_array: 0x{:08X}", agent_array);
  Ok(agent_array)
}

fn find_change_target_function(scanner: &Scanner) -> Result<(usize, usize)> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\AgentView\AvSelect.cpp"),
    Some(r"!(autoAgentId && !ManagerFindAgent(autoAgentId))"),
  )?;

  let change_target_fn = scanner.scan_back_to_function_start(address)?;
  log::debug!("Scanned change_target_function: 0x{:08X}", change_target_fn);

  let pattern = pattern![
    0x0f, 0x95, 0xc0, // SETNZ AL
    0x33, ?,          // XOR ..., ...
    0x89, ?, ?,       // MOV ..., ...
    0xa1,             // MOV EAX, [target_agent_id_ptr]
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let target_agent_id_ptr = unsafe { (address as *const usize).read_unaligned() };
  log::debug!("Scanned target_agent_id_ptr: 0x{:08X}", target_agent_id_ptr);

  Ok((change_target_fn, target_agent_id_ptr))
}

fn find_agent_skill_cast_ui_message_id(scanner: &Scanner, send_ui_message: usize) -> Result<usize> {
  let address = scanner.scan_for_assertion_call(r"P:\Code\Gw\AgentView\AvChar.cpp", r"0")?;
  let address = scanner.scan_for_function_call_back_from(send_ui_message, address)?;

  let pattern = pattern![
    0x68, ?, ?, ?, ?, // PUSH agent_skill_cast_ui_message_id
    0xe8,             // CALL send_ui_message
  ];
  let address = scanner.scan_back_from(&pattern, address)? + 1;
  let agent_skill_cast_ui_message_id = unsafe { (address as *const usize).read_unaligned() };

  log::debug!(
    "Scanned agent_skill_cast_ui_message_id: 0x{:08X}",
    agent_skill_cast_ui_message_id
  );
  Ok(agent_skill_cast_ui_message_id) // 0x10000027
}

fn find_skill_finished_ui_message_id(scanner: &Scanner, send_ui_message: usize) -> Result<usize> {
  let string = scanner.scan_for_string(r"Pending skill %u copy %d not found")?;
  let bytes = string.to_le_bytes();
  let pattern = pattern![
    0x68, bytes[0], bytes[1], bytes[2], bytes[3], // PUSH string
  ];
  let address = scanner.scan(&pattern)? + pattern.len();
  let address = scanner.scan_for_function_call_from(send_ui_message, address)?;

  #[rustfmt::skip]
  let pattern = pattern![
    0x6a, 0x00, // PUSH 0x0
    0x50,       // PUSH EAX
    0x68,       // PUSH skill_activated_ui_message_id
  ];

  let address = scanner.scan_back_from(&pattern, address)? + pattern.len();
  let skill_activated_ui_message_id = unsafe { (address as *const usize).read_unaligned() };
  let skill_finished_ui_message_id = skill_activated_ui_message_id + 0x2;

  log::debug!(
    "Scanned skill_finished_ui_message_id: 0x{:08X}",
    skill_finished_ui_message_id
  );
  Ok(skill_finished_ui_message_id) // 0x1000005d
}
