use bitflags::bitflags;

use crate::gw::models::{player::ProfessionId, skill::SkillId};

#[derive(super::Enum!)]
#[repr(u8)]
pub enum WeaponType {
  None,
  Bow,
  Axe,
  Hammer,
  Daggers,
  Scythe,
  Spear,
  Sword,
  ChaosCaster,
  ColdCaster,
  DarkCaster,
  EarthCaster,
  LightningCaster,
  FireCaster,
  HolyCaster,
}

bitflags! {
  #[repr(transparent)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub struct EffectsFlags: u32 {
    const Bleeding      = 1 << 0x00;
    const Conditioned   = 1 << 0x01;
    // const ?          = 1 << 0x02;
    const Crippled      = 1 << 0x03;
    const Dead          = 1 << 0x04;
    const DeepWounded   = 1 << 0x05;
    const Poisoned      = 1 << 0x06;
    const Enchanted     = 1 << 0x07;
    // const ?          = 1 << 0x08;
    // const ?          = 1 << 0x09;
    const DegenHexed    = 1 << 0x0a;
    const Hexed         = 1 << 0x0b;
    // const ?          = 1 << 0x0c;
    // const ?          = 1 << 0x0d;
    // const ?          = 1 << 0x0e;
    const WeaponSpelled = 1 << 0x0f;
  }
}

bitflags! {
  #[repr(transparent)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub struct TypeFlags: u32 {
    const InCombat        = 1 << 0x00;
    const HasQuest        = 1 << 0x01;
    const IsDead          = 1 << 0x03;
    const IsFemale        = 1 << 0x09;
    const HasBossGlow     = 1 << 0x0a;
    const IsHidingCape    = 1 << 0x0c;
    const IsInPartyWindow = 1 << 0x11;
    const IsSpawned       = 1 << 0x12;
    const IsPlayer        = 1 << 0x16;
  }
}

#[repr(C, packed)]
pub struct BaseAgent {
  /* 0000 */ _unknown0: [u8; 0x2c],
  /* 002c */ pub agent_id: u32,
  /* 0030 */ pub z: f32,
  /* 0034 */ width_1: f32,
  /* 0038 */ height_1: f32,
  /* 003c */ width_2: f32,
  /* 0040 */ height_2: f32,
  /* 0044 */ width_3: f32,
  /* 0048 */ height_3: f32,
  /* 004c */
  /** Rotation in radians from East (-pi to pi) */
  pub rotation_angle: f32,
  /* 0050 */
  /** cos of rotation_angle */
  pub rotation_cos: f32,
  /* 0054 */
  /** sin of rotation_angle */
  pub rotation_sin: f32,
  /* 0058 */ name_properties: u32,
  /* 005c */ ground: u32,
  /* 0060 */ _unknown1: [u8; 0x4],
  /* 0064 */ terrain_x: f32,
  /* 0068 */ terrain_y: f32,
  /* 006c */ terrain_z: f32,
  /* 0070 */ _unknown2: [u8; 0x4],
  /* 0074 */ pub x: f32,
  /* 0078 */ pub y: f32,
  /* 007c */ pub z_plane: u32,
  /* 0080 */ _unknown3: [u8; 0x1c],
  /* 009c */
  /** living = 0xdb, gadget = 0x200, item = 0x400 */
  pub agent_type: u32,
  /* 00a0 */ pub velocity_x: f32,
  /* 00a4 */ pub velocity_y: f32,
  /* 00a8 */ _unknown4: [u8; 0x1c],
  /* 00c4 */
}

#[repr(C, packed)]
pub struct Item {
  pub base: BaseAgent,
  /* +000 */ pub owner_id: u32,
  /* +004 */ pub item_id: u32,
  /* +008 */ _unknown0: [u8; 0x4],
  /* +00c */ pub extra_type: u32,
  /* +010 */
}

#[repr(C, packed)]
pub struct Gadget {
  pub base: BaseAgent,
  /* +000 */ _unknown0: [u8; 0x8],
  /* +008 */ pub extra_type: u32,
  /* +00c */ pub gadget_id: u32,
  /* +010 */ _unknown1: [u8; 0x10],
  /* +020 */
}

#[repr(C, packed)]
pub struct Agent {
  pub base: BaseAgent,
  /* +000 */ pub owner_id: u32,
  /* +004 */ _unknown0: [u8; 0x18],
  /* +01c */ pub animation_type: f32,
  /* +020 */ _unknown1: [u8; 0x8],
  /* +028 */
  /** The base attack speed of the last attack's weapon. 1.33 = axe, sword, daggers etc. */
  pub weapon_attack_speed: f32,
  /* +02c */
  /** Attack speed modifier of the last attack. 0.67 = 33% increase. */
  pub attack_speed_modifier: f32,
  /* +030 */
  /** Two of the same mob = same number */
  pub model_id: u16,
  /* +032 */
  /** NPC = 0x2000, Player = 0x3000 */
  pub agent_model_type: u16,
  /* +034 */ _unknown_2: [u8; 0x4],
  /* +038 */ pub equipment: usize,
  /* +03c */ _unknown_3: [u8; 0xe],
  /* +04a */ pub primary_profession: ProfessionId,
  /* +04b */ pub secondary_profession: ProfessionId,
  /* +04c */ pub level: u8,
  /* +04d */ team_id: u8,
  /* +04e */ _unknown_4: [u8; 0x6],
  /* +054 */ pub energy_regen: f32,
  /* +058 */ _unknown_5: [u8; 0x4],
  /* +05c */ pub energy: f32,
  /* +060 */ pub max_energy: u32,
  /* +064 */ _unknown_6: [u8; 0x4],
  /* +068 */ pub hp_pips: f32,
  /* +06c */ _unknown_7: [u8; 0x4],
  /* +070 */
  /** how full is the hp, between min 0 and max 1 */
  pub hp: f32,
  /* +074 */ pub max_hp: u32,
  /* +078 */ pub effects: EffectsFlags,
  /* +07c */ _unknown_8: [u8; 0x18],
  /* +094 */ pub model_state: u32,
  /* +098 */ pub type_map: TypeFlags,
  /* +09c */ _unknown_9: [u8; 0x14],
  /* +0b0 */ _unknown_10: [u8; 0xc],
  /* +0bc */ _unknown_11: [u8; 0x4],
  /* +0c0 */
  /** Unique number in instance that only works for players */
  pub login_number: u32,
  /* +0c4 */ pub animation_speed: f32,
  /* +0c8 */ pub animation_code: u32,
  /* +0cc */ pub animation_id: u32,
  /* +0d0 */ _unknown_12: [u8; 0x20],
  /* +0f0 */
  /** 0x1 = used lead attack, 0x2 = used offhand attack, 0x3 = used dual attack */
  pub dagger_status: u8,
  /* +0f1 */
  /** 0x1 = ally/non-attackable, 0x2 = neutral, 0x3 = enemy, 0x4 = spirit/pet, 0x5 = minion, 0x6 = npc/miniet */
  pub allegiance: u8,
  /* +0f2 */ pub weapon_type: WeaponType,
  /* +0f3 */ _unknown_13: u8,
  /* +0f4 */ pub skill: SkillId,
  /* +0f8 */ pub weapon_item_type: u8,
  /* +0f9 */ pub offhand_item_type: u8,
  /* +0fa */ pub weapon_item_id: u16,
  /* +0fc */ pub offhand_item_id: u16,
  /* +0fe */ _unknown_15: [u8; 0x2],
  /* +100 */
}

#[expect(unused)]
pub trait AsBaseAgent {
  #[expect(clippy::wrong_self_convention)]
  fn as_base(self) -> *const BaseAgent;
}

impl AsBaseAgent for *const Item {
  fn as_base(self) -> *const BaseAgent {
    self as _
  }
}

impl AsBaseAgent for *const Gadget {
  fn as_base(self) -> *const BaseAgent {
    self as _
  }
}

impl AsBaseAgent for *const Agent {
  fn as_base(self) -> *const BaseAgent {
    self as _
  }
}

pub fn into_item(base: *const BaseAgent) -> Result<*const Item, ()> {
  if base.is_null() {
    return Err(());
  }

  let agent_type = unsafe { (&raw const (*base).agent_type).read_unaligned() };
  if agent_type & 0x400 == 0 {
    Err(())
  } else {
    Ok(base as *const Item)
  }
}

pub fn into_gadget(base: *const BaseAgent) -> Result<*const Gadget, ()> {
  if base.is_null() {
    return Err(());
  }

  let agent_type = unsafe { (&raw const (*base).agent_type).read_unaligned() };
  if agent_type & 0x200 == 0 {
    Err(())
  } else {
    Ok(base as *const Gadget)
  }
}

pub fn into_agent(base: *const BaseAgent) -> Result<*const Agent, ()> {
  if base.is_null() {
    return Err(());
  }

  let agent_type = unsafe { (&raw const (*base).agent_type).read_unaligned() };
  if agent_type & 0xdb == 0 {
    Err(())
  } else {
    Ok(base as *const Agent)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<WeaponType>(), 0x1);
    assert_eq!(std::mem::size_of::<EffectsFlags>(), 0x4);
    assert_eq!(std::mem::size_of::<TypeFlags>(), 0x4);

    assert_eq!(std::mem::size_of::<BaseAgent>(), 0xc4);
    assert_eq!(std::mem::size_of::<Item>(), 0xd4);
    assert_eq!(std::mem::size_of::<Gadget>(), 0xe4);
    assert_eq!(std::mem::size_of::<Agent>(), 0x1c4);
  }
}
