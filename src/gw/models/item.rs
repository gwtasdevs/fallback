use crate::gw::models::Array;

#[repr(C, packed)]
pub struct Inventory {
  /* +000 */ pub storage_unused: *const Storage,
  /* +004 */ pub storage_backpack: *const Storage,
  /* +008 */ pub storage_belt_pouch_bag: *const Storage,
  /* +00c */ pub storage_bag_1: *const Storage,
  /* +010 */ pub storage_bag_2: *const Storage,
  /* +014 */ pub storage_equipment_pack: *const Storage,
  /* +018 */ pub storage_xunlai_materials: *const Storage,
  /* +01c */ pub storage_unclaimed_items: *const Storage,
  /* +020 */ pub storage_xunlai_1: *const Storage,
  /* +024 */ pub storage_xunlai_2: *const Storage,
  /* +028 */ pub storage_xunlai_3: *const Storage,
  /* +02c */ pub storage_xunlai_4: *const Storage,
  /* +030 */ pub storage_xunlai_5: *const Storage,
  /* +034 */ pub storage_xunlai_6: *const Storage,
  /* +038 */ pub storage_xunlai_7: *const Storage,
  /* +03c */ pub storage_xunlai_8: *const Storage,
  /* +040 */ pub storage_xunlai_9: *const Storage,
  /* +044 */ pub storage_xunlai_10: *const Storage,
  /* +048 */ pub storage_xunlai_11: *const Storage,
  /* +04c */ pub storage_xunlai_12: *const Storage,
  /* +050 */ pub storage_xunlai_13: *const Storage,
  /* +054 */ pub storage_xunlai_14: *const Storage,
  /* +058 */ pub storage_equipped_items: *const Storage,
  /* +05c */ pub bundle: *const Item,
  /* +060 */ pub is_xunlai_unlocked: u32,
  /* +064 */ pub weapon_set_0: *const Item,
  /* +068 */ pub offhand_set_0: *const Item,
  /* +06c */ pub weapon_set_1: *const Item,
  /* +070 */ pub offhand_set_1: *const Item,
  /* +074 */ pub weapon_set_2: *const Item,
  /* +078 */ pub offhand_set_2: *const Item,
  /* +07c */ pub weapon_set_3: *const Item,
  /* +080 */ pub offhand_set_3: *const Item,
  /* +084 */ pub active_weapon_set: u32,
  /* +088 */ _unknown0: [u8; 0x8],
  /* +090 */ pub gold_character: u32,
  /* +094 */ pub gold_xunlai: u32,
  /* +098 */ _unknown1: [u8; 0xc],
  /* +0a4 */
}

#[derive(super::Enum!)]
#[repr(u32)]
pub enum StorageType {
  None,
  Inventory,
  Equipped,
  NotCollected,
  Storage,
  MaterialStorage,
}

/** Called `Bag` in GWCA. */
#[repr(C, packed)]
pub struct Storage {
  /* +000 */ pub storage_type: StorageType,
  /* +004 */ index: u32,
  /* +008 */ _unknown0: u32,
  /* +00c */ container_item: u32,
  /* +010 */ item_count: u32,
  /* +014 */ inventory: *const Inventory,
  /* +018 */ pub items: Array<*const Item>,
  /* +028 */
}

#[derive(super::Enum!)]
#[repr(u8)]
pub enum ItemType {
  Salvage,
  Axe = 2,
  Bag,
  Boots,
  Bow,
  Bundle,
  Chestpiece,
  RuneMod,
  Usable,
  Dye,
  MaterialsZcoins,
  Offhand,
  Gloves,
  Hammer = 15,
  Headpiece,
  CCShards,
  Key,
  Leggings,
  GoldCoin,
  QuestItem,
  Wand,
  Shield = 24,
  Staff = 26,
  Sword,
  Kit = 29,
  Trophy,
  Scroll,
  Daggers,
  Present,
  Minipet,
  Scythe,
  Spear,
  Storybook = 43,
  Costume,
  CostumeHeadpiece,
}

#[repr(C, packed)]
pub struct DyeInfo {
  /* +000 */ tint: u8,
  /* +001 */ dye: u16,
  /* +003 */
}

#[repr(C, packed)]
pub struct ItemModifier {
  /* +000 */ pub arg: u16,
  /* +002 */ pub identifier: u16,
  /* +004 */
}

#[repr(C, packed)]
pub struct Item {
  /* +000 */ pub item_id: u32,
  /* +004 */ pub agent_id: u32,
  /* +008 */
  /** Only valid if item is in an equipped storage bag */
  pub storage_equipped: *const Storage,
  /* +00c */ pub storage: *const Storage,
  /* +010 */ pub modifiers: *const ItemModifier,
  /* +014 */ pub modifiers_size: usize,
  /* +018 */ customized: *const u16,
  /* +01c */ pub model_file_id: u32,
  /* +020 */ pub item_type: ItemType,
  /* +021 */ pub dye: DyeInfo,
  /* +024 */ pub value: u16,
  /* +026 */ _unknown0: u16,
  /* +028 */ _unknown1: u32,
  /* +02c */ pub model_id: u32,
  /* +030 */ info_string: *const u16,
  /* +034 */ pub encoded_name: *const u16,
  /* +038 */ pub encoded_complete_name: *const u16,
  /* +03c */ single_item_name: *const u16,
  /* +040 */ _unknown2: [u8; 0x14],
  /* +054 */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<Inventory>(), 0xa4);
    assert_eq!(std::mem::size_of::<StorageType>(), 0x4);
    assert_eq!(std::mem::size_of::<Storage>(), 0x28);
    assert_eq!(std::mem::size_of::<ItemModifier>(), 0x4);
    assert_eq!(std::mem::size_of::<Item>(), 0x54);
  }
}
