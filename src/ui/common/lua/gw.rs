use std::{sync::Arc, time::Duration};

use crate::{
  gw::{Gw, models},
  lua, lua_annotations, lua_enum, lua_func, read_ptr,
  ui::common::{
    lua::models::{
      self as lua_models,
      agent::{Agent, Gadget},
      item::Item,
    },
    utils,
  },
};

lua_enum!(models::agent::WeaponType, u8, "WeaponType");
lua_enum!(models::chat::ChatTextChannel, u32, "Channel");
lua_enum!(models::map::InstanceType, u32, "InstanceType");
lua_enum!(models::map::Map, u32, "Map");
lua_enum!(models::player::AttributeId, u8, "Attribute");
lua_enum!(models::player::ProfessionId, u8, "Profession");
lua_enum!(models::player::TitleId, u32, "Title");
lua_enum!(models::skill::SkillId, u32, "Skill");

#[derive(Clone, Copy, strum::AsRefStr, strum::EnumIter, strum::FromRepr, strum::IntoStaticStr)]
#[repr(u32)]
pub enum Range {
  Touch = 144,
  Adjacent = 166,
  Nearby = 252,
  InTheArea = 322,
  HoS = 360,
  ChainAggro = 700,
  Aggro = 1012,
  Casting = 1248,
  Spirit = 3000,
  Compass = 5020,
}
lua_enum!(Range, u32, "Range");

lua_func!(AgentPredicate, (agent: Agent) -> bool);
lua_func!(ItemPredicate, (item: Item) -> bool);
lua_func!(Predicate, () -> bool);

lua!(|gw_api: Arc<Gw>| {
  #[doc = "Guild Wars API."]
  gw = {
    agent = {
      change_target = async |agent_id: usize| -> Result<()> {
        gw_api.agent.change_target(agent_id);
        Ok(())
      },
      get_agent = async |agent_id: usize| -> Result<Option<Agent>> {
        if let Some(agent) = gw_api.agent.get_agent(agent_id) {
          Ok(Some(lua_models::agent::Agent::from_raw(agent, &gw_api)))
        } else {
          Ok(None)
        }
      },
      get_gadget = async |agent_id: usize| -> Result<Option<Gadget>> {
        if let Some(gadget) = gw_api.agent.get_gadget(agent_id) {
          Ok(Some(lua_models::agent::Gadget::from_raw(gadget, &gw_api)))
        } else {
          Ok(None)
        }
      },
      #[doc = "Get a list of all agent ids."]
      get_agent_ids = async || -> Result<Vec<u32>> {
        Ok(gw_api.agent.agents()
          .into_iter()
          .filter_map(|agent| {
            if agent.is_null() {
              None
            } else {
              let id = unsafe { (&raw const (*agent).agent_id).read_unaligned() };
              Some(id)
            }
          })
          .collect()
        )
      },
      get_player_agent_id = async || -> Result<usize> {
        Ok(gw_api.agent.get_player_agent_id())
      },
      get_player = async || -> Result<Option<Agent>> {
        if let Some(agent) = gw_api.agent.get_player() {
          Ok(Some(lua_models::agent::Agent::from_raw(agent, &gw_api)))
        } else {
          Ok(None)
        }
      },
      get_target_agent_id = async || -> Result<usize> {
        Ok(gw_api.agent.get_target_agent_id())
      },
      get_target_agent = async || -> Result<Option<Agent>> {
        if let Some(agent) = gw_api.agent.get_agent(gw_api.agent.get_target_agent_id()) {
          Ok(Some(lua_models::agent::Agent::from_raw(agent, &gw_api)))
        } else {
          Ok(None)
        }
      },
      #[doc = "Find an agent that matches the provided predicate function."]
      find = async |func: AgentPredicate| -> Result<Option<Agent>> {
        let result = gw_api.agent.agents()
          .into_iter()
          .filter_map(|base| models::agent::into_agent(base).ok())
          .map(|agent| lua_models::agent::Agent::from_raw(agent, &gw_api))
          .find(|agent| func.call(agent.clone()).unwrap_or(false));

        Ok(result)
      },
      #[doc = "Keep trying to find an agent that matches the provided predicate function until it is found. Returns the agent if found successfully, nil if it timed out. Defaults: timeout = 30seconds."]
      wait_until_found = async |func: AgentPredicate, timeout_secs: Option<u64>| -> Result<Option<Agent>> {
        let result = tokio::time::timeout(Duration::from_secs(timeout_secs.unwrap_or(30)), async {
          loop {
            tokio::time::sleep(Duration::from_millis(10)).await;

            for base_agent in gw_api.agent.agents().into_iter() {
              if let Ok(agent_ptr) = models::agent::into_agent(base_agent) {
                let agent = lua_models::agent::Agent::from_raw(agent_ptr, &gw_api);
                if func.call(agent).unwrap_or(false) {
                  let agent = lua_models::agent::Agent::from_raw(agent_ptr, &gw_api);
                  return agent;
                }
              }
            }
          }
        }).await;

        Ok(result.ok())
      },
      #[doc = "Get a list of agents that match the provided predicate function."]
      filter = async |func: AgentPredicate| -> Result<Vec<Agent>> {
        let agents = gw_api.agent.agents()
          .into_iter()
          .filter_map(|base| models::agent::into_agent(base).ok())
          .map(|agent| lua_models::agent::Agent::from_raw(agent, &gw_api))
          .filter(|agent| func.call(agent.clone()).unwrap_or(false))
          .collect::<Vec<_>>();

        Ok(agents)
      },
      #[doc = "Get the number of agents that match the provided predicate function."]
      count = async |func: AgentPredicate| -> Result<usize> {
        let mut count: usize = 0;
        for base_agent in gw_api.agent.agents().into_iter() {
          if let Ok(agent) = models::agent::into_agent(base_agent) {
            let agent = lua_models::agent::Agent::from_raw(agent, &gw_api);
            if func.call(agent).unwrap_or(false) {
              count += 1;
            }
          }
        }

        Ok(count)
      },
    },
    interact = {
      change_target = async |agent_id: usize| -> Result<()> {
        gw_api.agent.change_target(agent_id);
        Ok(())
      },
      drop_gold = async |amount: u32| -> Result<()> {
        let total_gold = gw_api.item.get_gold_on_character();
        if amount <= total_gold && gw_api.map.get_instance_type() == models::map::InstanceType::Explorable {
          gw_api.interact.drop_gold(amount);
        }
        Ok(())
      },
      interact_with_npc = async |agent_id: u32| -> Result<()> {
        gw_api.interact.interact_with_npc(agent_id as usize);
        Ok(())
      },
      #[doc = "Open the currently targeted chest using a key or lockpick."]
      open_chest = async |use_key: bool| -> Result<()> {
        gw_api.interact.open_chest(use_key);
        Ok(())
      },
      open_xunlai_chest = async || -> Result<()> {
        gw_api.interact.open_xunlai_chest();
        Ok(())
      },
      send_dialog = async |dialog_id: u32| -> Result<()> {
        gw_api.interact.send_dialog(dialog_id);
        Ok(())
      },
      send_first_dialog = async || -> Result<()> {
        gw_api.interact.send_first_dialog();
        Ok(())
      },
      send_chat = async |channel: models::chat::ChatTextChannel, message: String| -> Result<()> {
        gw_api.chat.send_chat(channel, &message);
        Ok(())
      },
      #[doc = "Send a raw packet to the server."]
      send_packet = async |packet: mlua::Variadic<usize>| -> Result<()> {
        gw_api.packet.send_sized_packet(packet.to_vec());
        Ok(())
      },
    },
    map = {
      get_instance_type = async || -> Result<models::map::InstanceType> {
        Ok(gw_api.map.get_instance_type())
      },
      get_map_id = async || -> Result<Option<models::map::Map>> {
        Ok(gw_api.map.get_map_id())
      },
      travel = async |map_id: models::map::Map| -> Result<()> {
        gw_api.map.travel(map_id);
        Ok(())
      },
    },
    player = {
      get_agent = async || -> Result<Option<Agent>> {
        if let Some(agent) = gw_api.agent.get_player() {
          Ok(Some(lua_models::agent::Agent::from_raw(agent, &gw_api)))
        } else {
          Ok(None)
        }
      },
      #[doc = "Returns true if the player has this skill on the skillbar."]
      has_skill = async |skill_id: models::skill::SkillId| -> Result<bool> {
        Ok(gw_api.player.has_skill(skill_id))
      },
      use_skill = async |skill_id: models::skill::SkillId| -> Result<()> {
        gw_api.player.use_skill(skill_id);
        Ok(())
      },
      #[doc = "Returns true if the player is currently affected by this effect."]
      has_effect = async |skill_id: models::skill::SkillId| -> Result<bool> {
        Ok(gw_api.player.has_effect(skill_id))
      },
      #[doc = "Get how long before this skill is ready to use in milliseconds."]
      get_skill_cooldown = async |skill_id: models::skill::SkillId| -> Result<Option<u32>> {
        Ok(gw_api.player.get_skill_cooldown(skill_id))
      },
      #[doc = "Get how long before this effect ends in milliseconds."]
      get_effect_remaining_duration = async |skill_id: models::skill::SkillId| -> Result<u32> {
        Ok(gw_api.player.get_effect_remaining_duration(skill_id))
      },
      #[doc = "Equip an item by its unique id, which can be found in the Info window."]
      equip_item = async |unique_id: String| -> Result<()> {
        let (model_id, modifiers) = utils::decode_unique_item_id(&unique_id)
          .map_err(|_| mlua::Error::RuntimeError(format!("Invalid unique item id: {}", unique_id)))?;
        if let Some(item) = gw_api.item.find_item(model_id, modifiers) && let Some(item_id) = read_ptr!(item, .item_id) {
          gw_api.item.equip_item(item_id);
        } else {
          gw_api.chat.print_chat(models::chat::ChatChannel::Warning, "Could not equip item, the item was not found.");
        }

        Ok(())
      },
      use_item = async |model_id: u32| -> Result<()> {
        gw_api.item.use_item_by_model_id(model_id);
        Ok(())
      },
      remove_title = async || -> Result<()> {
        gw_api.player.remove_active_title();
        Ok(())
      },
      #[doc = "Remove and re-apply the current title."]
      reapply_title = async || -> Result<()> {
        if let Some(title) = gw_api.player.get_active_title() {
          gw_api.player.remove_active_title();
          // tokio::time::sleep(Duration::from_millis(50)).await;
          gw_api.player.set_active_title(title);
        }
        Ok(())
      },
      set_title = async |title_id: models::player::TitleId| -> Result<()> {
        gw_api.player.set_active_title(title_id);
        Ok(())
      },
      #[doc = "Start moving to the specified coordinates."]
      start_moving = async |x: f32, y: f32| -> Result<()> {
        gw_api.player.move_to(x, y);
        Ok(())
      },
      #[doc = "Move to the specified coordinates and wait until the player is within range. Returns true if the player arrived successfully, false if it timed out. Defaults: range = 100, timeout = 30seconds."]
      move_to = async |x: f32, y: f32, range: Option<f32>, timeout_secs: Option<u64>| -> Result<bool> {
        let result = tokio::time::timeout(Duration::from_secs(timeout_secs.unwrap_or(30)), async {
          gw_api.player.move_to(x, y);
          let range = range.unwrap_or(100.);
          let range_sq = range * range;

          loop {
            tokio::time::sleep(Duration::from_millis(10)).await;
            if let Some(player) = gw_api.agent.get_player() {
              let player_x = unsafe { (&raw const (*player).base.x).read_unaligned() };
              let player_y = unsafe { (&raw const (*player).base.y).read_unaligned() };

              let dist_sq = (player_x - x)*(player_x - x) + (player_y - y)*(player_y - y);
              if dist_sq <= range_sq {
                return true;
              }
            }
          }
        }).await;

        Ok(result.unwrap_or(false))
      },
      #[doc = "Gets the player's distance from the specified coordinates."]
      distance_from = async |x: f32, y: f32| -> Result<f32> {
        if let Some(player) = gw_api.agent.get_player() {
          let player_x = unsafe { (&raw const (*player).base.x).read_unaligned() };
          let player_y = unsafe { (&raw const (*player).base.y).read_unaligned() };

          let dist_sq = (player_x - x)*(player_x - x) + (player_y - y)*(player_y - y);
          return Ok(dist_sq.sqrt());
        }

        Ok(0f32)
      },
      get_primary_profession = async || -> Result<Option<models::player::ProfessionId>> {
        if let Some(player) = gw_api.agent.get_player() {
          let profession = unsafe { (&raw const (*player).primary_profession).read_unaligned() };
          Ok(Some(profession))
        } else {
          Ok(None)
        }
      },
      get_secondary_profession = async || -> Result<Option<models::player::ProfessionId>> {
        if let Some(player) = gw_api.agent.get_player() {
          let profession = unsafe { (&raw const (*player).secondary_profession).read_unaligned() };
          Ok(Some(profession))
        } else {
          Ok(None)
        }
      },
      get_party_size = async || -> Result<u32> {
        Ok(gw_api.player.get_party_size())
      },
    },
    util = {
      log = async |message: String| -> Result<()> {
        gw_api.chat.print_chat(models::chat::ChatChannel::Advisory, &message);
        Ok(())
      },
      #[doc = "Wait for the specified number of milliseconds."]
      wait = async |duration_ms: u32| -> Result<()> {
        tokio::time::sleep(Duration::from_millis(duration_ms.into())).await;
        Ok(())
      },
      #[doc = "Wait until the provided predicate function returns true."]
      wait_until = async |func: Predicate| -> Result<()> {
        while !func.call(())? {
          tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
      },
    },
  }
});
