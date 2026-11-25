#[derive(super::Enum!)]
#[repr(u8)]
pub enum AttributeId {
  FastCasting,
  IllusionMagic,
  DominationMagic,
  InspirationMagic,
  BloodMagic,
  DeathMagic,
  SoulReaping,
  Curses,
  AirMagic,
  EarthMagic,
  FireMagic,
  WaterMagic,
  EnergyStorage,
  HealingPrayers,
  SmitingPrayers,
  ProtectionPrayers,
  DivineFavor,
  Strength,
  AxeMastery,
  HammerMastery,
  Swordsmanship,
  Tactics,
  BeastMastery,
  Expertise,
  WildernessSurvival,
  Marksmanship,
  DaggerMastery = 0x1d,
  DeadlyArts = 0x1e,
  ShadowArts = 0x1f,
  Communing = 0x20,
  RestorationMagic = 0x21,
  ChannelingMagic = 0x22,
  CriticalStrikes = 0x23,
  SpawningPower = 0x24,
  SpearMastery = 0x25,
  Command = 0x26,
  Motivation = 0x27,
  Leadership = 0x28,
  ScytheMastery = 0x29,
  WindPrayers = 0x2a,
  EarthPrayers = 0x2b,
  Mysticism = 0x2c,
  None = 0xff,
}

#[derive(super::Enum!)]
#[repr(u8)]
pub enum ProfessionId {
  None,
  Warrior,
  Ranger,
  Monk,
  Necromancer,
  Mesmer,
  Elementalist,
  Assassin,
  Ritualist,
  Paragon,
  Dervish,
}

#[derive(super::Enum!)]
#[repr(u32)]
pub enum TitleId {
  Hero = 0,
  TyrianCartographer = 1,
  CanthanCartographer = 2,
  Gladiator = 3,
  Champion = 4,
  Kurzick = 5,
  Luxon = 6,
  Drunkard = 7,
  Survivor = 9,
  MaxedTitles = 10,
  ProtectorOfTyria = 13,
  ProtectorOfCantha = 14,
  Lucky = 15,
  Unlucky = 16,
  Sunspear = 17,
  ElonianCartographer = 18,
  ProtectorOfElona = 19,
  Lightbringer = 20,
  LegendaryDefenderOfAscalon = 21,
  Commander = 22,
  Gamer = 23,
  TyriaEliteSkillHunter = 24,
  VanquisherTyria = 25,
  CanthanEliteSkillHunter = 26,
  VanquisherCantha = 27,
  ElonianEliteSkillHunter = 28,
  VanquisherElona = 29,
  LegendaryCartographer = 30,
  LegendaryGuardian = 31,
  LegendarySkillHunter = 32,
  LegendaryVanquisher = 33,
  SweetTooth = 34,
  GuardianOfTyria = 35,
  GuardianOfCantha = 36,
  GuardianOfElona = 37,
  Asura = 38,
  Deldrimor = 39,
  EbonVanguard = 40,
  Norn = 41,
  MasterOfTheNorth = 42,
  PartyAnimal = 43,
  Zaishen = 44,
  TreasureHunter = 45,
  Wisdom = 46,
  Codex = 47,
  None = 0xff,
}

#[repr(C, packed)]
pub struct Player {
  /* +000 */ pub agent_id: u32,
  /* +004 */ _unknown0: [u8; 0x20],
  /* +024 */ pub encoded_name: *const u16,
  /* +028 */ pub name: *const u16,
  /* +02c */ pub party_leader_player_number: u32,
  /* +030 */ pub active_title_tier_id: u32,
  /* +034 */ _unknown1: u32,
  /* +038 */ pub player_number: u32,
  /* +03c */ pub party_size: u32,
  /* +040 */ _unknown2: [u8; 0x10],
  /* +050 */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<AttributeId>(), 0x1);
    assert_eq!(std::mem::size_of::<ProfessionId>(), 0x1);
    assert_eq!(std::mem::size_of::<Player>(), 0x50);
  }
}
