use crate::gw::models::skill::SkillId;

#[repr(C, packed)]
pub struct TooltipInfo {
  /* +000 */ bit_field: u32,
  /* +004 */ _unknown0: u32,
  /* +008 */ pub payload: *const u32,
  /* +00c */ tooltip_type: u32,
  /* +010 */ _unknown1: [u8; 0x8],
  /* +018 */
}

impl TooltipInfo {
  pub fn is_item(&self) -> bool {
    // 0x8 is used for item, attribute, title tooltips and more.
    // Item tooltip has 2 item_id values, second is always 0xff
    self.tooltip_type == 0x8 && unsafe { self.payload.add(1).read_unaligned() } == 0xff
  }

  pub fn is_skill(&self) -> bool {
    self.tooltip_type == 0x14
  }

  pub fn is_weapon_set(&self) -> bool {
    self.tooltip_type == 0xc
  }
}

pub fn get_tooltip_item_id(tooltip: *const TooltipInfo) -> Option<u32> {
  if tooltip.is_null() {
    return None;
  }

  let tooltip_type = unsafe { (&raw const (*tooltip).tooltip_type).read_unaligned() };
  let payload = unsafe { (&raw const (*tooltip).payload).read_unaligned() };
  if payload.is_null() {
    return None;
  }

  if tooltip_type == 0x8 && unsafe { payload.add(1).read_unaligned() } == 0xff {
    let item_id = unsafe { payload.read_unaligned() };
    Some(item_id)
  } else {
    None
  }
}

#[expect(unused)]
pub fn get_tooltip_skill_id(tooltip: *const TooltipInfo) -> Option<SkillId> {
  if tooltip.is_null() {
    return None;
  }

  let tooltip_type = unsafe { (&raw const (*tooltip).tooltip_type).read_unaligned() };
  if tooltip_type == 0x14 {
    let payload = unsafe { (&raw const (*tooltip).payload).read_unaligned() };
    let skill_id = unsafe { payload.read_unaligned() };
    SkillId::from_repr(skill_id)
  } else {
    None
  }
}
