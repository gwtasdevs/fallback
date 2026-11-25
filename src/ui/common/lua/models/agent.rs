use crate::{
  gw::{
    Gw,
    models::{
      self,
      agent::{EffectsFlags, TypeFlags, WeaponType},
    },
  },
  lua_struct,
};

lua_struct! {
  #[doc = "Represents a game agent"]
  struct Agent {
    #[doc = "The agent's ID."]
    agent_id: u32,

    is_ally: bool,
    is_enemy: bool,
    is_allied_minion: bool,
    is_allied_spirit: bool,

    is_bleeding: bool,
    is_crippled: bool,
    is_dead: bool,
    is_deep_wounded: bool,
    is_degen_hexed: bool,
    is_enchanted: bool,
    is_hexed: bool,
    is_poisoned: bool,
    is_weapon_spelled: bool,

    is_in_combat: bool,
    is_in_party_window: bool,

    #[doc = "How full the agent's HP is, ranging from 0 to 1."]
    hp: f32,

    #[doc = "HP regeneration pips, ranges from -10 to 10."]
    hp_pips: f32,

    model_id: u16,
    weapon_type: WeaponType,
    x: f32,
    y: f32,

    #[doc = "The agent name can take some time to load. It will be `nil` until loaded."]
    name: Option<String>,
  }
}

impl Agent {
  pub fn from_raw(agent: *const models::agent::Agent, gw: &Gw) -> Self {
    unsafe {
      let base_agent = &raw const (*agent).base;
      let agent_id = (&raw const (*base_agent).agent_id).read_unaligned();
      let allegiance = (&raw const (*agent).allegiance).read_unaligned();
      let effects = (&raw const (*agent).effects).read_unaligned();
      let type_map = (&raw const (*agent).type_map).read_unaligned();

      Agent {
        agent_id,

        is_ally: allegiance == 0x1 || allegiance == 0x4 || allegiance == 0x5 || allegiance == 0x6,
        is_enemy: allegiance == 0x3,
        is_allied_minion: allegiance == 0x5,
        is_allied_spirit: allegiance == 0x4,

        is_bleeding: effects.contains(EffectsFlags::Bleeding),
        is_crippled: effects.contains(EffectsFlags::Crippled),
        is_dead: effects.contains(EffectsFlags::Dead),
        is_deep_wounded: effects.contains(EffectsFlags::DeepWounded),
        is_degen_hexed: effects.contains(EffectsFlags::DegenHexed),
        is_enchanted: effects.contains(EffectsFlags::Enchanted),
        is_hexed: effects.contains(EffectsFlags::Hexed),
        is_poisoned: effects.contains(EffectsFlags::Poisoned),
        is_weapon_spelled: effects.contains(EffectsFlags::WeaponSpelled),

        is_in_combat: type_map.contains(TypeFlags::InCombat),
        is_in_party_window: type_map.contains(TypeFlags::IsInPartyWindow),

        hp: (&raw const (*agent).hp).read_unaligned(),
        hp_pips: (&raw const (*agent).hp_pips).read_unaligned(),
        model_id: (&raw const (*agent).model_id).read_unaligned(),
        weapon_type: (&raw const (*agent).weapon_type).read_unaligned(),
        x: (&raw const (*base_agent).x).read_unaligned(),
        y: (&raw const (*base_agent).y).read_unaligned(),
        name: gw.agent.try_get_agent_name(agent_id),
      }
    }
  }
}

lua_struct! {
  #[doc = "Represents a game gadget agent"]
  struct Gadget {
    #[doc = "The agent's ID."]
    agent_id: u32,
    gadget_id: u32,
    x: f32,
    y: f32,
    name: Option<String>,
  }
}

impl Gadget {
  pub fn from_raw(gadget: *const models::agent::Gadget, gw: &Gw) -> Self {
    unsafe {
      let base_agent = &raw const (*gadget).base;
      let agent_id = (&raw const (*base_agent).agent_id).read_unaligned();

      Gadget {
        agent_id,
        gadget_id: (&raw const (*gadget).gadget_id).read_unaligned(),
        x: (&raw const (*base_agent).x).read_unaligned(),
        y: (&raw const (*base_agent).y).read_unaligned(),
        name: gw.agent.try_get_agent_name(agent_id),
      }
    }
  }
}
