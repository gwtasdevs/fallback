#[derive(super::Enum!)]
#[repr(u32)]
pub enum ChatTextChannel {
  All,
  Guild,
  Team,
  Trade,
  Alliance,
  Whisper,
  Command,
}

impl From<ChatTextChannel> for u16 {
  fn from(value: ChatTextChannel) -> Self {
    match value {
      ChatTextChannel::All => '!' as u16,
      ChatTextChannel::Guild => '@' as u16,
      ChatTextChannel::Team => '#' as u16,
      ChatTextChannel::Trade => '$' as u16,
      ChatTextChannel::Alliance => '%' as u16,
      ChatTextChannel::Whisper => '"' as u16,
      ChatTextChannel::Command => '/' as u16,
    }
  }
}

#[derive(super::Enum!)]
#[repr(u32)]
pub enum ChatChannel {
  Alliance = 0,
  Allies = 1,
  All = 3,
  Moderator = 5,
  Emote = 6,
  Warning = 7,
  Guild = 9,
  Global = 10,
  Group = 11,
  Trade = 12,
  Advisory = 13,
  Whisper = 14,
}
