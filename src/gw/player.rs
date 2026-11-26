use std::sync::Arc;

use anyhow::{Context, Result};
use windows::Win32::System::SystemInformation::GetTickCount;

use crate::{
  gw::{
    models::{self, Array},
    scan::Scanner,
  },
  parallel, pattern, read_ptr,
};

pub struct Player {
  agent: Arc<crate::gw::agent::Agent>,
  packet: Arc<crate::gw::packet::Packet>,
  thread: Arc<crate::gw::thread::Thread>,

  skill_timer_ptr: usize,
  use_skill_fn: unsafe extern "cdecl" fn(agent_id: u32, slot: u32, target_agent_id: u32, call_target: u32) -> (),
  move_to_fn: unsafe extern "cdecl" fn(pos: *const models::Position) -> (),
}

impl Player {
  pub fn is_player_loaded(&self) -> bool {
    self.agent.get_player().is_some()
  }

  pub fn get_skill_bar(&self) -> Option<*const models::skill::SkillBar> {
    if let Some(skill_bars) = read_ptr!(self.thread.get_game_context(), .world.skill_bars) {
      let player_agent_id = self.agent.get_player_agent_id() as u32;

      for skill_bar in skill_bars.iter() {
        if skill_bar.is_null() {
          continue;
        }

        let agent_id = unsafe { (&raw const (*skill_bar).agent_id).read_unaligned() };
        if agent_id == 0 {
          continue;
        }

        if agent_id == player_agent_id {
          return Some(skill_bar);
        }
      }
    }

    None
  }

  pub fn get_skill_slot(&self, skill_id: models::skill::SkillId) -> Option<usize> {
    if let Some(skill_bar) = self.get_skill_bar()
      && let Some(skills) = read_ptr!(skill_bar, .skills)
    {
      for (i, skill) in skills.iter().enumerate() {
        if { skill.skill_id } == skill_id {
          return Some(i);
        }
      }
    }

    None
  }

  pub fn has_skill(&self, skill_id: models::skill::SkillId) -> bool {
    self.get_skill_cooldown(skill_id).is_some()
  }

  pub fn get_skill_cooldown(&self, skill_id: models::skill::SkillId) -> Option<u32> {
    if let Some(skill_bar) = self.get_skill_bar()
      && let Some(skills) = read_ptr!(skill_bar, .skills)
    {
      for skill in skills {
        if { skill.skill_id } == skill_id {
          return Some(skill.recharge);
        }
      }
    }

    None
  }

  pub fn has_effect(&self, skill_id: models::skill::SkillId) -> bool {
    if let Some(effects) = self.get_effects() {
      for effect in effects.into_iter() {
        if { effect.skill_id } == skill_id {
          return true;
        }
      }
    }

    false
  }

  pub fn get_effect_remaining_duration(&self, skill_id: models::skill::SkillId) -> u32 {
    if let Some(effects) = self.get_effects() {
      for effect in effects.into_iter() {
        if { effect.skill_id } == skill_id {
          let tick_count = unsafe { GetTickCount() };
          let skill_timer = unsafe { (self.skill_timer_ptr as *const u32).read_unaligned() };
          let time_elapsed = tick_count.wrapping_add(skill_timer).wrapping_sub(effect.timestamp);
          let duration = (effect.duration * 1000.) as u32;
          let result = if duration > time_elapsed {
            duration.wrapping_sub(time_elapsed)
          } else {
            u32::MAX
          };
          return result;
        }
      }
    }

    0
  }

  pub fn get_buffs(&self) -> Option<Array<models::skill::Buff>> {
    if let Some(context) = self.get_effects_context() {
      read_ptr!(context, .buffs)
    } else {
      None
    }
  }

  pub fn get_effects(&self) -> Option<Array<models::skill::Effect>> {
    if let Some(context) = self.get_effects_context() {
      read_ptr!(context, .effects)
    } else {
      None
    }
  }

  fn get_effects_context(&self) -> Option<*const models::skill::AgentEffectsContext> {
    if let Some(party_effects) = read_ptr!(self.thread.get_game_context(), .world.party_effects_context) {
      let player_agent_id = self.agent.get_player_agent_id() as u32;
      for agent_effects in party_effects.iter() {
        if agent_effects.is_null() {
          continue;
        }

        if let Some(agent_id) = read_ptr!(agent_effects, .agent_id) {
          if agent_id == 0 {
            continue;
          }
          if agent_id == player_agent_id {
            return Some(agent_effects);
          }
        }
      }
    }

    None
  }

  pub fn get_party_size(&self) -> u32 {
    if let Some(players) = read_ptr!(self.thread.get_game_context(), .party.player_party.players) {
      players.size
    } else {
      0
    }
  }

  pub fn use_skill(&self, skill_id: models::skill::SkillId) {
    if self.is_player_loaded() {
      let player_agent_id = self.agent.get_player_agent_id() as u32;

      if let Some(slot) = self.get_skill_slot(skill_id) {
        let func = self.use_skill_fn;
        let target_agent_id = 0;
        let call_target = 0;

        self.thread.fire_and_forget(move || {
          unsafe { func(player_agent_id, slot as u32, target_agent_id, call_target) };
        });
      }
    }
  }

  pub fn move_to(&self, x: f32, y: f32) {
    if self.is_player_loaded() {
      let func = self.move_to_fn;

      self.thread.fire_and_forget(move || {
        let position = models::Position { x, y, z_plane: 0 };
        unsafe { func(&raw const position) };
      });
    }
  }

  pub fn get_active_title(&self) -> Option<models::player::TitleId> {
    let player_number = read_ptr!(self.thread.get_game_context(), .character.player_number)?;
    let tier_id = read_ptr!(self.thread.get_game_context(), .world.players[player_number].active_title_tier_id)?;
    let titles = read_ptr!(self.thread.get_game_context(), .world.titles)?;

    titles
      .into_iter()
      .position(|title| title.title_tier_id == tier_id)
      .and_then(|title_id| models::player::TitleId::from_repr(title_id as u32))
  }

  pub fn set_active_title(&self, title: models::player::TitleId) {
    self.packet.send_sized_packet(vec![0x58, title as usize]);
  }

  pub fn remove_active_title(&self) {
    self.packet.send_packet(0x59);
  }
}

pub fn init(
  scanner: &Scanner,
  agent: Arc<crate::gw::agent::Agent>,
  packet: Arc<crate::gw::packet::Packet>,
  thread: Arc<crate::gw::thread::Thread>,
) -> Result<Player> {
  let (skill_timer_ptr, use_skill, move_to) = parallel![
    || find_skill_timer_pointer(scanner).context("Could not find skill_timer pointer"),
    || find_use_skill_function(scanner).context("Could not find use_skill function."),
    || find_move_to_function(scanner).context("Could not find move_to function."),
  ]?;

  Ok(Player {
    agent,
    packet,
    thread,

    skill_timer_ptr,
    use_skill_fn: unsafe { std::mem::transmute(use_skill) },
    move_to_fn: unsafe { std::mem::transmute(move_to) },
  })
}

fn find_skill_timer_pointer(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Base\Os\Win32\Exe\ExeTime.cpp"), Some(r"result"))?;

  let pattern = pattern![
    0x83, 0x3d, ?, ?, ?, ?, 0x00, // CMP [skill_timer_ptr], 0x0
  ];

  let address = scanner.scan_from(&pattern, address)? + 2;
  let skill_timer_ptr = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned skill_timer_ptr: 0x{:08X}", skill_timer_ptr);
  Ok(skill_timer_ptr)
}

fn find_use_skill_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_wide_string("/.hotkey 8 4")?;
  let bytes = address.to_le_bytes();

  let pattern = pattern![
    0x68, bytes[0], bytes[1], bytes[2], bytes[3], // PUSH address
    0xe8, ?, ?, ?, ?,                             // CALL ...
  ];

  let address = scanner.scan(&pattern)? + pattern.len();

  let pattern = pattern![
    0xe8, // CALL use_skill_function
  ];

  let address = scanner.scan_from(&pattern, address)?;
  let use_skill_function = scanner.follow_call(address)?;

  log::debug!("Scanned use_skill_function: 0x{:08X}", use_skill_function);
  Ok(use_skill_function)
}

fn find_move_to_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Ui\Game\GmCoreAction.cpp"),
    Some("action < WORLD_ACTIONS"),
  )?;

  let pattern = pattern![
    0xff, 0x24, 0x9d, // JMP [switch_table + EBX*4]
  ];

  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let switch_table = unsafe { (address as *const usize).read_unaligned() };
  let case_1 = unsafe { ((switch_table + 4) as *const usize).read_unaligned() };

  #[rustfmt::skip]
  let pattern = pattern![
    0x3d, 0xdb, 0x00, 0x00, 0x00, // CMP EAX, 0xdb
    0x75,                         // JNZ ...
  ];

  let address = scanner.scan_from(&pattern, case_1)?;
  let address = scanner.follow_call(address + pattern.len() - 1)?;

  let pattern = pattern![
    0x50,             // PUSH EAX
    0xe8, ?, ?, ?, ?, // CALL move_to_function
    0x83, 0xc4, 0x0c, // ADD ESP, 0xc
  ];
  let address = scanner.scan_from(&pattern, address)?;
  let move_to_function = scanner.follow_call(address + 1)?;

  log::debug!("Scanned move_to_function: 0x{:08X}", move_to_function);
  Ok(move_to_function)
}
