use crate::gw::models::{
  Array,
  item::{Inventory, Item, Storage},
  map::Map,
  player::Player,
  skill::{AgentEffectsContext, SkillBar},
};

#[repr(C, packed)]
pub struct GameContext {
  /* +000 */ _unknown0: u32,
  /* +004 */ _unknown1: u32,
  /* +008 */ pub agent: *const AgentContext,
  /* +00c */ _unknown2: u32,
  /* +010 */ _unknown3: u32,
  /* +014 */ pub map: *const MapContext,
  /* +018 */ text_parser: u32,
  /* +01c */ _unknown4: u32,
  /* +020 */ _unknown5: u32,
  /* +024 */ _unknown6: u32,
  /* +028 */ account: u32,
  /* +02c */ pub world: *const WorldContext,
  /* +030 */ cinematic: u32,
  /* +034 */ _unknown7: u32,
  /* +038 */ pub gadget: *const GadgetContext,
  /* +03c */ guild: u32,
  /* +040 */ pub item: *const ItemContext,
  /* +044 */ pub character: *const CharContext,
  /* +048 */ _unknown8: u32,
  /* +04c */ pub party: *const PartyContext,
  /* +050 */ _unknown9: u32,
  /* +054 */ _unknown10: u32,
  /* +058 */ trade: u32,
  /* +05c */
}

#[repr(C, packed)]
pub struct GadgetContext {
  /* +000 */ pub gadget_info: Array<GadgetInfo>,
  /* +010 */
}

#[repr(C, packed)]
pub struct GadgetInfo {
  /* +000 */ _unknown0: [u8; 0xc],
  /* +00c */ pub encoded_name: *const u16,
  /* +010 */
}

#[repr(C, packed)]
pub struct ItemContext {
  /* +000 */ _unknown0: [u8; 0x24],
  /* +024 */ pub storage: Array<*const Storage>,
  /* +034 */ _unknown1: [u8; 0x84],
  /* +0b8 */ pub items: Array<*const Item>,
  /* +0c8 */ _unknown2: [u8; 0xc],
  /* +0d4 */ inventory_table: [u8; 0x24],
  /* +0f8 */ pub player_inventory: *const Inventory,
  /* +0fc */ _unknown3: [u8; 0x10],
  /* +10c */
}

#[repr(C, packed)]
pub struct AgentContext {
  /* +000 */ _unknown0: [u8; 0x98],
  /* +098 */ pub agent_summaries: Array<AgentSummary>,
  /* +0a8 */ _unknown1: [u8; 0x104],
  /* +1ac */ instance_timer: u32,
  /* +1b0 */
}

#[repr(C, packed)]
pub struct AgentSummary {
  /* +000 */ _unknown0: [u8; 0x8],
  /* +008 */ pub info: *const AgentSummaryInfo,
  /* +00c */
}

#[repr(C, packed)]
pub struct AgentSummaryInfo {
  /* +000 */ _unknown0: [u8; 0x8],
  /* +008 */ pub gadget_id: u32,
  /* +00c */ _unknown1: u32,
  /* +010 */ pub encoded_name: *const u16,
  /* +014 */ _unknown2: [u8; 0x8],
  /* +01c */
}

#[repr(C, packed)]
pub struct CharContext {
  /* +000 */ _unknown0: [u8; 0x64],
  /* +064 */ player_uuid: [u32; 4],
  /* +074 */ pub player_name: [u16; 0x14],
  /* +09c */ _unknown1: [u8; 0xfc],
  /* +198 */ map_id: u32,
  /* +19c */ is_explorable: u32,
  /* +1a0 */ _unknown2: [u8; 0x8c],
  /* +22c */ pub district_id: u32,
  /* +230 */ pub current_language_id: u32,
  /* +234 */ observed_map_id: Map,
  /* +238 */ pub current_map_id: Map,
  /* +23c */ _unknown3: [u8; 0x74],
  /* +2b0 */ pub player_number: u32,
  /* +2b4 */ _unknown4: [u8; 0x11c],
  /* +3d0 */ pub player_email: [u16; 0x40],
  /* +450 */
}

#[repr(C, packed)]
pub struct MapContext {
  /* +000 */ map_boundaries: [f32; 5],
  /* +014 */ _unknown0: [u8; 0x60],
  /* +074 */ pub map_boundary: *const MapBoundaryContext,
  /* +078 */
}

#[repr(C, packed)]
pub struct MapBoundaryContext {
  /* +000 */ pub pathing_maps_container: *const PathingMapsContainer,
  /* +004 */ pathing_map_block: Array<u32>,
  /* +014 */ pub total_trapezoid_count: u32,
  /* +018 */ _unknown0: [u8; 0x58],
  /* +070 */
}

#[repr(C, packed)]
pub struct PathingMapsContainer {
  /* +000 */ _unknown0: [u8; 0x18],
  /* +018 */ pub pathing_maps: Array<PathingMap>,
  /* +028 */
}

#[repr(C, packed)]
pub struct PathingMap {
  /* +000 */ pub z_plane: u32,
  /* +004 */ _unknown0: [u8; 0x10],
  /* +014 */ pub trapezoid_count: u32,
  /* +018 */ pub trapezoids: *const PathingTrapezoid,
  /* +01c */ _unknown1: [u8; 0x38],
  /* +054 */
}

#[repr(C, packed)]
pub struct PathingTrapezoid {
  /* +000 */ pub id: u32,
  /* +004 */ pub adjacent: [*const PathingTrapezoid; 4],
  /* +014 */ _unknown0: u32,
  /* +018 */ pub xtl: f32,
  /* +01c */ pub xtr: f32,
  /* +020 */ pub yt: f32,
  /* +024 */ pub xbl: f32,
  /* +028 */ pub xbr: f32,
  /* +02c */ pub yb: f32,
  /* +030 */
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct PathingPoint {
  /* +000 */ pub x: f32,
  /* +004 */ pub y: f32,
  /* +008 */ pub z_plane: u32,
  /* +00c */ pub trapezoid: *const PathingTrapezoid,
  /* +010 */
}

unsafe impl Send for PathingPoint {}

#[repr(C, packed)]
pub struct WorldContext {
  /* +000 */ _unknown0: [u8; 0x508],
  /* +508 */ pub party_effects_context: Array<AgentEffectsContext>,
  /* +518 */ _unknown1: [u8; 0x4c],
  /* +564 */ pub mission_objectives: Array<MissionObjective>,
  /* +574 */ _unknown2: [u8; 0x17c],
  /* +6f0 */ pub skill_bars: Array<SkillBar>,
  /* +700 */ _unknown3: [u8; 0xcc],
  /* +7cc */ pub agent_infos: Array<AgentInfo>,
  /* +7dc */ _unknown4: [u8; 0x20],
  /* +7fc */ pub npcs: Array<Npc>,
  /* +80c */ pub players: Array<Player>,
  /* +81c */ pub titles: Array<Title>,
  /* +82c */ pub title_tiers: Array<TitleTier>,
  /* +83c */ _unknown5: [u8; 0x18],
  /* +854 */
}

#[repr(C, packed)]
pub struct Title {
  /* +000 */ _unknown0: u32,
  /* +004 */ pub current_points: u32,
  /* +008 */ pub title_tier_id: u32,
  /* +00c */ _unknown1: [u8; 0x1c],
  /* +028 */
}

#[repr(C, packed)]
pub struct TitleTier {
  /* +000 */ _unknown0: u32,
  /* +004 */ tier_number: u32,
  /* +008 */ pub encoded_name: *const u16,
  /* +00c */
}

#[repr(C, packed)]
pub struct Npc {
  /* +000 */ _unknown0: [u8; 0x20],
  /* +020 */ pub encoded_name: *const u16,
  /* +024 */ _unknown1: [u8; 0xc],
  /* +030 */
}

#[repr(C, packed)]
pub struct AgentInfo {
  /* +000 */ _unknown0: [u8; 0x34],
  /* +034 */ pub encoded_name: *const u16,
  /* +038 */
}

#[repr(C, packed)]
pub struct MissionObjective {
  /* +000 */ pub objective_id: u32,
  /* +004 */ pub encoded_string: *const u16,
  /* +008 */ pub objective_type: u32,
  /* +00c */
}

#[repr(C, packed)]
pub struct PartyContext {
  /* +000 */ _unknown0: [u8; 0x40],
  /* +040 */ pub parties: Array<*const PartyInfo>,
  /* +050 */ _unknown1: u32,
  /* +054 */ pub player_party: *const PartyInfo,
  /* +058 */ _unknown2: [u8; 0x78],
  /* +0d0 */
}

#[repr(C, packed)]
pub struct PartyInfo {
  /* +000 */ pub party_id: u32,
  /* +004 */ pub players: Array<PlayerPartyMember>,
  /* +014 */ henchmen: Array<u32>,
  /* +024 */ heroes: Array<u32>,
  /* +034 */ others: Array<u32>,
  /* +044 */ _unknown0: [u8; 0x40],
  /* +084 */
}

#[repr(C, packed)]
pub struct PlayerPartyMember {
  /* +000 */ pub player_id: u32,
  /* +004 */ pub called_target_agent_id: u32,
  /* +008 */ pub state: u32,
  /* +00c */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<GameContext>(), 0x5c);
    assert_eq!(std::mem::size_of::<GadgetContext>(), 0x10);
    assert_eq!(std::mem::size_of::<GadgetInfo>(), 0x10);
    assert_eq!(std::mem::size_of::<ItemContext>(), 0x10c);
    assert_eq!(std::mem::size_of::<AgentContext>(), 0x1b0);
    assert_eq!(std::mem::size_of::<AgentSummary>(), 0xc);
    assert_eq!(std::mem::size_of::<CharContext>(), 0x450);
    assert_eq!(std::mem::size_of::<MapContext>(), 0x78);
    assert_eq!(std::mem::size_of::<MapBoundaryContext>(), 0x70);
    assert_eq!(std::mem::size_of::<PathingMapsContainer>(), 0x28);
    assert_eq!(std::mem::size_of::<PathingMap>(), 0x54);
    assert_eq!(std::mem::size_of::<PathingTrapezoid>(), 0x30);
    assert_eq!(std::mem::size_of::<PathingPoint>(), 0x10);
    assert_eq!(std::mem::size_of::<WorldContext>(), 0x854);
    assert_eq!(std::mem::size_of::<Title>(), 0x28);
    assert_eq!(std::mem::size_of::<TitleTier>(), 0xc);
    assert_eq!(std::mem::size_of::<Npc>(), 0x30);
    assert_eq!(std::mem::size_of::<AgentInfo>(), 0x38);
    assert_eq!(std::mem::size_of::<MissionObjective>(), 0xc);
    assert_eq!(std::mem::size_of::<PartyContext>(), 0xD0);
    assert_eq!(std::mem::size_of::<PartyInfo>(), 0x84);
    assert_eq!(std::mem::size_of::<PlayerPartyMember>(), 0xc);
  }
}
