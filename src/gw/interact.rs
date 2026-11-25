use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};

use crate::gw::hook::Hook;
use crate::gw::models::ui_message::DialogButtonUiMessage;
use crate::gw::scan::Scanner;
use crate::{parallel, pattern};

pub struct Interact {
  thread: Arc<crate::gw::thread::Thread>,
  ui: Arc<crate::gw::ui::Ui>,

  drop_gold_fn: unsafe extern "cdecl" fn(amount: u32) -> (),
  interact_npc_fn: unsafe extern "cdecl" fn(agent_id: usize, call_target: usize) -> (),
  open_chest_fn: unsafe extern "cdecl" fn(key_type: usize) -> (),
  send_dialog_fn: unsafe extern "cdecl" fn(dialog_id: u32) -> (),
  show_xunlai_chest_ui_message_id: usize,
  available_dialogs: Arc<Mutex<Vec<u32>>>,
  _cleanup: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl Drop for Interact {
  fn drop(&mut self) {
    if let Some(cleanup) = self._cleanup.take() {
      cleanup();
    }
  }
}

impl Interact {
  pub fn drop_gold(&self, amount: u32) {
    let func = self.drop_gold_fn;
    unsafe { func(amount) };
  }

  pub fn interact_with_npc(&self, agent_id: usize) {
    let func = self.interact_npc_fn;

    self.thread.fire_and_forget(move || {
      unsafe { func(agent_id, 0) };
    });
  }

  pub fn open_chest(&self, use_key: bool) {
    let func = self.open_chest_fn;

    self.thread.fire_and_forget(move || {
      unsafe { func(if use_key { 1 } else { 2 }) };
    });
  }

  pub fn open_xunlai_chest(&self) {
    self.ui.send_ui_message(self.show_xunlai_chest_ui_message_id, 0, 0);
  }

  pub fn send_dialog(&self, dialog_id: u32) {
    let func = self.send_dialog_fn;

    self.thread.fire_and_forget(move || {
      unsafe { func(dialog_id) };
    });
  }

  pub fn send_first_dialog(&self) {
    if let Ok(available_dialogs) = self.available_dialogs.lock()
      && let Some(dialog_id) = available_dialogs.first()
    {
      let dialog_id = *dialog_id;
      drop(available_dialogs);
      self.send_dialog(dialog_id);
    }
  }
}

struct UiInteractionMessage {
  _action_type: u32,
  message_id: usize,
  _w_param: usize,
}

pub fn init(scanner: &Scanner, thread: Arc<crate::gw::thread::Thread>, ui: Arc<crate::gw::ui::Ui>) -> Result<Interact> {
  let npc_dialog_ui_callback =
    find_npc_dialog_ui_callback_function(scanner).context("Could not find npc_dialog_ui_callback function")?;

  let (
    open_chest_fn,
    show_xunlai_chest_ui_message_id,
    send_dialog_fn,
    dialog_button_ui_message_id,
    drop_gold_fn,
    interact_npc_fn,
  ) = parallel![
    || find_open_chest_function(scanner, npc_dialog_ui_callback).context("Could not find open_chest function."),
    || find_show_xunlai_chest_ui_message_id(scanner, ui.send_ui_message_fn)
      .context("Could not find show_xunlai_chest ui message id."),
    || find_send_dialog_function(scanner).context("Could not find send_dialog functions."),
    || find_dialog_button_ui_message_id(scanner, npc_dialog_ui_callback)
      .context("Could not find dialog_button ui message id."),
    || find_drop_gold_function(scanner).context("Could not find drop_gold function."),
    || find_interact_npc_function(scanner).context("Could not find interact_npc function."),
  ]?;

  let dialog_body_ui_message_id = dialog_button_ui_message_id + 3;

  let available_dialogs = Arc::new(Mutex::new(vec![]));

  let available_dialogs_ref = available_dialogs.clone();
  let dialog_body_ui_message_hook_id =
    ui.register_send_ui_message_hook::<_, ()>(dialog_body_ui_message_id, move |_| {
      if let Ok(mut available_dialogs) = available_dialogs_ref.lock() {
        available_dialogs.clear();
      }
    });

  let available_dialogs_ref = available_dialogs.clone();
  let dialog_button_ui_message_hook_id =
    ui.register_send_ui_message_hook::<_, DialogButtonUiMessage>(dialog_button_ui_message_id, move |message| {
      if !message.is_null() {
        let dialog_id = unsafe { (&raw const (*message).dialog_id).read_unaligned() };
        if let Ok(mut available_dialogs) = available_dialogs_ref.lock() {
          available_dialogs.push(dialog_id);
        }
      }
    });

  let npc_dialog_ui_callback_hook = Hook::new(npc_dialog_ui_callback)?;
  let destroy_frame_ui_message_id = ui.destroy_frame_ui_message_id;
  let available_dialogs_ref = available_dialogs.clone();
  let npc_dialog_ui_callback_hook_id = npc_dialog_ui_callback_hook.register_hook(move |reg| {
    let esp = reg.esp as *const usize;
    let message_ptr = unsafe { esp.add(1).read_unaligned() } as *const UiInteractionMessage;
    // let w_param = unsafe { esp.add(2).read_unaligned() };
    // let l_param = unsafe { esp.add(3).read_unaligned() };

    if !message_ptr.is_null() {
      let message_id = unsafe { (&raw const (*message_ptr).message_id).read_unaligned() };
      match message_id {
        id if id == destroy_frame_ui_message_id => {
          if let Ok(mut available_dialogs) = available_dialogs_ref.lock() {
            available_dialogs.clear();
          }
        }
        _ => {}
      }
    }
  });

  Ok(Interact {
    thread,
    ui: ui.clone(),

    drop_gold_fn: unsafe { std::mem::transmute(drop_gold_fn) },
    interact_npc_fn: unsafe { std::mem::transmute(interact_npc_fn) },
    open_chest_fn: unsafe { std::mem::transmute(open_chest_fn) },
    send_dialog_fn: unsafe { std::mem::transmute(send_dialog_fn) },
    show_xunlai_chest_ui_message_id,
    available_dialogs,
    _cleanup: Some(Box::new(move || {
      ui.deregister_send_ui_message_hook(&dialog_body_ui_message_hook_id);
      ui.deregister_send_ui_message_hook(&dialog_button_ui_message_hook_id);
      npc_dialog_ui_callback_hook.deregister_hook(&npc_dialog_ui_callback_hook_id);
    })),
  })
}

fn find_npc_dialog_ui_callback_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Ui\Game\GmNpc.cpp"),
    Some(r"interactMsg.codedText && interactMsg.codedText[0]"),
  )?;
  let npc_dialog_ui_callback = scanner.scan_back_to_function_start(address)?;

  log::debug!("Scanned npc_dialog_ui_callback: 0x{:08X}", npc_dialog_ui_callback);
  Ok(npc_dialog_ui_callback)
}

fn find_open_chest_function(scanner: &Scanner, npc_dialog_ui_callback_fn: usize) -> Result<usize> {
  let pattern = pattern![
    0x0f, 0x87, // JA ...
  ];
  let address = scanner.scan_from(&pattern, npc_dialog_ui_callback_fn)?;
  let address = scanner.follow_call(address)?;

  #[rustfmt::skip]
  let pattern = pattern![
    0x8b, 0x09, // MOV ECX, [ECX]
    0xe8,       // CALL ...
  ];
  let address = scanner.scan_from(&pattern, address)? + 2;
  let address = scanner.follow_call(address)?;

  let pattern = pattern![
    0x3d, ?, ?, ?, ?,       // CMP EAX, dialog_button_ui_message_id
    0x0f, 0x87, ?, ?, ?, ?, // JA ...
    ?,                      // PUSH ...
    ?,                      // PUSH ...
    0x0f, 0x84,             // JZ ...
  ];

  let address = scanner.scan_from(&pattern, address)?;
  let address = scanner.follow_call(address + pattern.len() - 2)?;

  let pattern = pattern![
    0xe8, ?, ?, ?, ?, // CALL open_chest_function
    0x83, 0xc4, 0x04, // ADD ESP, 0x4
  ];

  let address = scanner.scan_back_from(&pattern, address)?;
  let open_chest_function = scanner.follow_call(address)?;
  log::debug!("Scanned open_chest_function: 0x{:08X}", open_chest_function);

  Ok(open_chest_function)
}

fn find_show_xunlai_chest_ui_message_id(scanner: &Scanner, send_ui_message: usize) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Char\Cli\ChCliApi.cpp"),
    Some("context->accumIntList[0].Count() == context->accumIntList[1].Count()"),
  )?;

  let address = scanner.scan_for_function_call_back_from(send_ui_message, address)?;
  let address = scanner.scan_for_function_call_back_from(send_ui_message, address)?;
  let address = scanner.scan_back_to_function_start(address)?;

  let pattern = pattern![
    0xff, 0x24, 0x85, // JMP [switch_table + EAX*4]
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let switch_table = unsafe { (address as *const usize).read_unaligned() };
  let case_0 = unsafe { (switch_table as *const usize).read_unaligned() };

  let pattern = pattern![
    0x68, ?, 0x00, 0x00, 0x10, // PUSH show_xunlai_chest_ui_message_id
  ];
  let address = scanner.scan_from(&pattern, case_0)? + 1;
  let show_xunlai_chest_ui_message_id = unsafe { (address as *const usize).read_unaligned() };
  log::debug!(
    "Scanned show_xunlai_chest_ui_message_id: 0x{:08X}",
    show_xunlai_chest_ui_message_id
  );

  Ok(show_xunlai_chest_ui_message_id) // 0x10000040
}

fn find_send_dialog_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Ui\Game\Vendor\VnUnlockHero.cpp"),
    Some(r"m_selection"),
  )?;

  let pattern = pattern![
    0x00, 0x00, 0x00, 0x0e, // OR ..., 0xe000000
    ?,                      // PUSH ...
    0xe8,                   // CALL dialog_check_function
  ];

  let address = scanner.scan_from(&pattern, address)? + pattern.len() - 1;
  let address = scanner.follow_call(address)?;

  let pattern = pattern![
    0xe9, ?, ?, ?, ?, // JMP send_dialog_function
    0x5d,             // POP EBP
    0xc3,             // RET
  ];

  let address = scanner.scan_from(&pattern, address)?;
  let send_dialog_function = scanner.follow_call(address)?;
  log::debug!("Scanned send_dialog_function: 0x{:08X}", send_dialog_function);

  Ok(send_dialog_function)
}

fn find_dialog_button_ui_message_id(scanner: &Scanner, npc_dialog_ui_callback: usize) -> Result<usize> {
  let pattern = pattern![
    0x0f, 0x87, // JA ...
  ];
  let address = scanner.scan_from(&pattern, npc_dialog_ui_callback)?;
  let address = scanner.follow_call(address)?;

  #[rustfmt::skip]
  let pattern = pattern![
    0x8b, 0x09, // MOV ECX, [ECX]
    0xe8,       // CALL ...
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len() - 1;
  let address = scanner.follow_call(address)?;

  let pattern = pattern![
    0x3d, ?, ?, ?, ?, // CMP EAX, dialog_button_ui_message_id
    0x0f, 0x87,       // JA ...
  ];

  let address = scanner.scan_from(&pattern, address)? + 1;

  let dialog_button_ui_message_id = unsafe { (address as *const usize).read_unaligned() };
  log::debug!(
    "Scanned dialog_button_ui_message_id: 0x{:08X}",
    dialog_button_ui_message_id
  );

  Ok(dialog_button_ui_message_id)
}

fn find_drop_gold_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Gw\Ui\Game\GmView.cpp"), Some(r"param.notifyData"))?;

  let pattern = pattern![
    0xe8, ?, ?, ?, ?, // CALL drop_gold_fn
    0x83, 0xc4, 0x04, // ADD ESP, 0x4
  ];

  let address = scanner.scan_from(&pattern, address)?;
  let drop_gold_fn = scanner.follow_call(address)?;
  log::debug!("Scanned drop_gold_fn: 0x{:08X}", drop_gold_fn);

  Ok(drop_gold_fn)
}

fn find_interact_npc_function(scanner: &Scanner) -> Result<usize> {
  let addresses = scanner.scan_for_assertions(
    Some(r"P:\Code\Gw\Ui\Game\GmCoreAction.cpp"),
    Some(r"action < WORLD_ACTIONS"),
  )?;

  let address = addresses
    .into_iter()
    .find(|address| scanner.scan_back_to_function_start_until(*address, 0x20).is_ok())
    .ok_or(anyhow!("No match found."))?;

  let pattern = pattern![
    0x83, 0xfb, 0x05,       // CMP EBX, 0x5
    0x0f, 0x87, ?, ?, ?, ?, // JA ...
    0xff, 0x24, 0x9d,       // JMP [switch_table + EBX*4]
  ];

  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let switch_table = unsafe { (address as *const *const usize).read_unaligned() };
  let case_2 = unsafe { switch_table.add(2).read_unaligned() };

  let pattern = pattern![
    0xe8, // CALL interact_npc_fn
  ];

  let address = scanner.scan_from(&pattern, case_2)?;
  let interact_npc_fn = scanner.follow_call(address)?;
  log::debug!("Scanned interact_npc_fn: 0x{:08X}", interact_npc_fn);

  Ok(interact_npc_fn)
}
