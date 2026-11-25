use std::sync::Arc;

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::{
  gw::{
    hook::Hook,
    models::{
      chat::{ChatChannel, ChatTextChannel},
      ui_message::ChatUiMessage,
    },
    scan::Scanner,
  },
  parallel, pattern,
};

pub struct Chat {
  ui: Arc<crate::gw::ui::Ui>,

  send_chat_fn: extern "cdecl" fn(buffer: *const u16, agent_id: u32) -> (),
  write_chat_ui_message_id: usize,
  send_chat_hook: Hook<'static>,
}

impl Chat {
  pub fn send_chat(&self, channel: ChatTextChannel, message: &str) {
    let mut buffer: Vec<u16> = vec![channel.into()];
    buffer.extend(message.encode_utf16().take(120));
    buffer.push(0);

    (self.send_chat_fn)(buffer.as_ptr(), 0);
  }

  pub fn print_chat(&self, channel: ChatChannel, message: &str) {
    let message = ChatUiMessage::new(channel, message);
    self.ui.send_ui_message(self.write_chat_ui_message_id, message, 0);
  }

  pub fn register_send_chat_hook<F>(&self, func: F) -> Uuid
  where
    F: Fn(*const u16, usize) + Send + Sync + 'static,
  {
    self.send_chat_hook.register_hook(move |reg| {
      let esp = reg.esp as *const usize;
      let message = unsafe { esp.add(1).read_unaligned() } as *const u16;
      let agent_id = unsafe { esp.add(2).read_unaligned() };

      func(message, agent_id);
    })
  }

  pub fn deregister_send_chat_hook(&self, id: &Uuid) -> bool {
    self.send_chat_hook.deregister_hook(id)
  }
}

pub fn init(scanner: &Scanner, ui: Arc<crate::gw::ui::Ui>) -> Result<Chat> {
  let (send_chat, write_chat_ui_message_id) = parallel![
    || find_send_chat_function(scanner).context("Could not find send_chat function."),
    || find_write_chat_ui_message_id(scanner, ui.send_ui_message_fn)
      .context("Could not find write_chat ui message id."),
  ]?;

  let send_chat_hook = Hook::new(send_chat)?;

  Ok(Chat {
    ui,

    send_chat_fn: unsafe { std::mem::transmute(send_chat) },
    write_chat_ui_message_id,
    send_chat_hook,
  })
}

fn find_send_chat_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_wide_string("/.hotkey 8 4")?;
  let bytes = address.to_le_bytes();

  #[rustfmt::skip]
  let pattern = pattern![
    0x68, bytes[0], bytes[1], bytes[2], bytes[3], // PUSH address
    0xe8,                                         // CALL ...
  ];
  let address = scanner.scan(&pattern)? + pattern.len() - 1;
  let address = scanner.follow_call(address)?;

  let pattern = pattern![
    0x5d, // POP EBP
    0xe9, // JMP send_chat_function
  ];
  let address = scanner.scan_from(&pattern, address)? + 1;
  let send_chat_function = scanner.follow_call(address)?;

  log::debug!("Scanned send_chat_function: 0x{:08X}", send_chat_function);
  Ok(send_chat_function)
}

fn find_write_chat_ui_message_id(scanner: &Scanner, send_ui_message_fn: usize) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Comm\CommCli.cpp"),
    Some(r"TextValidateCoded(codedText)"),
  )?;
  let address = scanner.scan_for_function_call_from(send_ui_message_fn, address)?;

  let pattern = pattern![
    0x68, ?, ?, 0x00, 0x10, // PUSH write_chat_ui_message_id
  ];

  let address = scanner.scan_back_from(&pattern, address)? + 1;
  let write_chat_ui_message_id = unsafe { (address as *const usize).read_unaligned() };
  log::debug!("Scanned write_chat_ui_message_id: 0x{:08X}", write_chat_ui_message_id);

  Ok(write_chat_ui_message_id)
}
