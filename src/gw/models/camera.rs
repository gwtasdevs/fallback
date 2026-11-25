#[repr(C, packed)]
pub struct Camera {
  /* 0000 */ pub player_agent_id: u32,
  /* 0004 */ _unknown0: [u8; 0x14],
  /* 0018 */ pub yaw: f32,
  /* 001c */ pub pitch: f32,
  /* 0020 */ _unknown1: [u8; 0x58],
  /* 0078 */ pub position_x: f32,
  /* 007c */ pub position_y: f32,
  /* 0080 */ _unknown2: [u8; 0x28],
  /* 00a8 */ pub look_at_target_x: f32,
  /* 00ac */ pub look_at_target_y: f32,
  /* 00b0 */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<Camera>(), 0xb0);
  }
}
