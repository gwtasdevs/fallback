use crate::gw::models::{chat::ChatChannel, map::Map, skill::SkillId};

#[repr(C, packed)]
pub struct ChatUiMessage {
  /* +000 */ pub channel: ChatChannel,
  /* +004 */ pub message: *const u16,
  /* +008 */ pub channel_2: ChatChannel,
  /* +00c */
  /** This field does not exist on the GW struct, it is here only to hold ownership of the message string. */
  _owned_message: Vec<u16>,
}

unsafe impl Send for ChatUiMessage {}

#[expect(unused)]
impl ChatUiMessage {
  pub fn new(channel: ChatChannel, message: &str) -> Self {
    let encoded = format!("\u{108}\u{107}{}\u{1}\u{0}", message)
      .encode_utf16()
      .collect::<Vec<_>>();

    Self {
      channel,
      message: encoded.as_ptr(),
      channel_2: channel,
      _owned_message: encoded,
    }
  }

  pub fn new_with_sender(channel: ChatChannel, sender: &str, message: &str) -> Self {
    let has_link_in_message = message.contains("<a=1>");
    let has_markup = has_link_in_message || message.contains("<c=");

    let encoded_sender = format!("\u{108}\u{107}{}\u{1}", sender);
    let encoded_message = format!("\u{108}\u{107}{}\u{1}", message);

    let encoded = if has_link_in_message {
      format!(
        "\u{108}\u{107}<a=2>\u{1}\u{2}{}\u{2}\u{108}\u{107}</a>\u{1}\u{2}\u{108}\u{107}: \u{1}\u{2}{}\u{0}",
        encoded_sender, encoded_message,
      )
    } else if has_markup {
      format!(
        "\u{108}\u{107}<a=1>\u{1}\u{2}{}\u{2}\u{108}\u{107}</a>\u{1}\u{2}\u{108}\u{107}: \u{1}\u{2}{}\u{0}",
        encoded_sender, encoded_message,
      )
    } else {
      format!(
        "\u{76b}\u{10a}{}\u{1}\u{10b}{}\u{1}\u{0}",
        encoded_sender, encoded_message,
      )
    };
    let encoded = encoded.encode_utf16().collect::<Vec<_>>();

    Self {
      channel,
      message: encoded.as_ptr(),
      channel_2: channel,
      _owned_message: encoded,
    }
  }
}

#[repr(C, packed)]
pub struct DialogButtonUiMessage {
  /* +000 */ button_icon: u32,
  /* +004 */ message: *const u16,
  /* +008 */ pub dialog_id: u32,
  /* +00c */
  /** Default: 0xFFFFFFF */
  skill_id: u32,
  /* +010 */
}

#[repr(C, packed)]
pub struct MapLoadedUiMessage {}

#[repr(C, packed)]
pub struct MapTravelUiMessage {
  /* +000 */ pub map_id: Map,
  /* +004 */ pub region: u32,
  /* +008 */ pub language: u32,
  /* +00c */ pub district: u32,
  /* +010 */
}

#[repr(C, packed)]
pub struct SkillActivatedUiMessage {
  /* 0000 */ pub agent_id: u32,
  /* 0004 */ pub skill_id: SkillId,
  /* 0008 */ pub activation_time: f32,
  /* 000c */ _unknown0: f32,
  /* 0010 */
}

#[repr(C, packed)]
pub struct SkillFinishedUiMessage {
  /* 0000 */ pub agent_id: u32,
  /* 0004 */ pub skill_id: SkillId,
  /* 0008 */ _unknown0: u32,
  /* 000c */ pub cool_down_seconds: f32,
  /* 0010 */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<DialogButtonUiMessage>(), 0x10);
    assert_eq!(std::mem::size_of::<MapTravelUiMessage>(), 0x10);
    assert_eq!(std::mem::size_of::<SkillActivatedUiMessage>(), 0x10);
    assert_eq!(std::mem::size_of::<SkillFinishedUiMessage>(), 0x10);
  }
}
