use std::{
  collections::HashMap,
  ptr,
  sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::{
  gw::{hook::Hook, models, scan::Scanner},
  parallel, pattern,
};

pub struct Ui {
  thread: Arc<crate::gw::thread::Thread>,

  pub send_ui_message_fn: usize,
  #[expect(unused)]
  send_frame_ui_message_fn: usize,
  async_decode_string_fn: unsafe extern "cdecl" fn(
    encoded_str: usize,
    callback: unsafe extern "cdecl" fn(w_param: usize, result: *const u16),
    w_param: usize,
  ) -> usize,
  send_ui_message_hook: Hook<'static>,
  pub destroy_frame_ui_message_id: usize,
  current_tooltip_ptr: usize,
  encoded_string_cache: Arc<Mutex<HashMap<Vec<u16>, Option<String>>>>,
}

impl Drop for Ui {
  fn drop(&mut self) {
    log::debug!("Dropping Ui");
  }
}

impl Ui {
  pub fn send_ui_message<T: Send + 'static>(&self, message_id: usize, message: T, l_param: usize) {
    let func: extern "cdecl" fn(message_id: usize, message: usize, l_param: usize) -> () =
      unsafe { std::mem::transmute(self.send_ui_message_fn) };

    self.thread.fire_and_forget(move || {
      func(message_id, &raw const message as usize, l_param);
    });
  }

  pub fn register_send_ui_message_hook<F, T>(&self, id: usize, func: F) -> Uuid
  where
    F: Fn(*const T) + Send + Sync + 'static,
  {
    self.send_ui_message_hook.register_hook(move |reg| {
      let esp = reg.esp as *const usize;
      let message_id = unsafe { esp.add(1).read_unaligned() };
      if message_id != id {
        return;
      }

      let w_param = unsafe { esp.add(2).read_unaligned() } as *const T;
      // let l_param = unsafe { esp.add(3).read_unaligned() };

      func(w_param);
    })
  }

  pub fn deregister_send_ui_message_hook(&self, id: &Uuid) -> bool {
    self.send_ui_message_hook.deregister_hook(id)
  }

  pub fn get_current_tooltip(&self) -> *const models::ui::TooltipInfo {
    let ptr = self.current_tooltip_ptr as *const *const *const models::ui::TooltipInfo;
    if !ptr.is_null() {
      let ptr = unsafe { ptr.read_unaligned() };
      if !ptr.is_null() {
        return unsafe { ptr.read_unaligned() };
      }
    }

    ptr::null()
  }

  pub fn try_clone_encoded_string(&self, encoded_string: *const u16) -> Option<Vec<u16>> {
    if encoded_string.is_null() {
      return None;
    }

    let mut len = 0;
    while unsafe { encoded_string.add(len).read_unaligned() } != 0 {
      len += 1;
    }

    if len == 0 {
      return None;
    }

    // the null terminator is included
    Some(unsafe { std::slice::from_raw_parts(encoded_string, len + 1) }.to_vec())
  }

  pub fn try_get_decoded_string(&self, encoded_string: *const u16) -> Option<String> {
    if let Some(encoded_string) = self.try_clone_encoded_string(encoded_string) {
      if let Ok(cache) = self.encoded_string_cache.lock()
        && let Some(cached) = cache.get(&encoded_string)
      {
        return cached.clone();
      }

      let func = self.async_decode_string_fn;
      let cache = self.encoded_string_cache.clone();
      let encoded_string_ref = encoded_string.clone();
      self.thread.fire_and_forget(move || {
        let data_ptr = Box::into_raw(Box::new((encoded_string.clone(), cache))) as usize;
        unsafe {
          func(
            encoded_string.as_ptr() as usize,
            async_decode_string_cache_callback,
            data_ptr,
          )
        };
      });

      // if the data was already available, the callback gets called before async_decode_string_fn returns,
      // so check the cache to see if the value is there now.
      if let Ok(cache) = self.encoded_string_cache.lock()
        && let Some(cached) = cache.get(&encoded_string_ref)
      {
        return cached.clone();
      }

      // decode requested, but we do not have the result yet, so return None for now
    }

    None
  }

  pub async fn decode_string(&self, encoded_string: Vec<u16>) -> Option<String> {
    if encoded_string.is_empty() {
      return None;
    }

    if let Ok(cache) = self.encoded_string_cache.lock()
      && let Some(cached) = cache.get(&encoded_string)
    {
      return cached.clone();
    }

    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    let mut sender = Some(tx);

    let func = self.async_decode_string_fn;
    let encoded_string_ref = encoded_string.clone();
    self
      .thread
      .execute(move || {
        unsafe {
          func(
            encoded_string_ref.as_ptr() as usize,
            async_decode_string_callback,
            &raw mut sender as usize,
          )
        };
      })
      .await;

    let decoded = rx.await.unwrap_or(None);

    if let Ok(mut cache) = self.encoded_string_cache.lock() {
      if let Some(decoded) = decoded {
        cache.insert(encoded_string, Some(decoded.clone()));
        Some(decoded)
      } else {
        cache.remove(&encoded_string);
        None
      }
    } else {
      decoded
    }
  }
}

unsafe extern "cdecl" fn async_decode_string_callback(data_ptr: usize, decoded_str_ptr: *const u16) {
  let data_ptr = data_ptr as *mut Option<tokio::sync::oneshot::Sender<Option<String>>>;
  if data_ptr.is_null() {
    return;
  }
  let sender = unsafe { &mut *data_ptr };
  if let Some(sender) = sender.take() {
    if !decoded_str_ptr.is_null() {
      let mut len = 0;
      while unsafe { decoded_str_ptr.add(len).read_unaligned() } != 0 {
        len += 1;
      }

      if len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(decoded_str_ptr, len) };
        let _ = sender.send(Some(String::from_utf16_lossy(slice)));
        return;
      }
    }

    let _ = sender.send(None);
  }
}

unsafe extern "cdecl" fn async_decode_string_cache_callback(data_ptr: usize, decoded_str_ptr: *const u16) {
  let data_ptr = data_ptr as *mut (Vec<u16>, Arc<Mutex<HashMap<Vec<u16>, Option<String>>>>);
  if data_ptr.is_null() {
    return;
  }
  let (encoded_string, cache) = *unsafe { Box::from_raw(data_ptr) };

  if let Ok(mut cache) = cache.lock()
    && !decoded_str_ptr.is_null()
  {
    let mut len = 0;
    while unsafe { decoded_str_ptr.add(len).read_unaligned() } != 0 {
      len += 1;
    }

    if len > 0 {
      let slice = unsafe { std::slice::from_raw_parts(decoded_str_ptr, len) };
      cache.insert(encoded_string.clone(), Some(String::from_utf16_lossy(slice)));
    }
  }
}

pub fn init(scanner: &Scanner, thread: Arc<crate::gw::thread::Thread>) -> Result<Ui> {
  let (
    send_ui_message_fn,
    send_frame_ui_message_fn,
    destroy_frame_ui_message_id,
    current_tooltip_ptr,
    async_decode_string_fn,
  ) = parallel![
    || find_send_ui_message_function(scanner).context("Could not find send_ui_message function."),
    || find_send_frame_ui_message_function(scanner).context("Could not find send_frame_ui_message function."),
    || find_destroy_frame_ui_message_id(scanner).context("Could not find destroy_frame ui message id."),
    || find_current_tooltip_pointer(scanner).context("Could not find current_tooltip pointer."),
    || find_async_decode_string_function(scanner).context("Could not find async_decode_string function."),
  ]?;

  let send_ui_message_hook = Hook::new(send_ui_message_fn)?;

  Ok(Ui {
    thread,

    send_ui_message_fn,
    send_frame_ui_message_fn,
    async_decode_string_fn: unsafe { std::mem::transmute(async_decode_string_fn) },
    send_ui_message_hook,
    destroy_frame_ui_message_id,
    current_tooltip_ptr,
    encoded_string_cache: Arc::new(Mutex::new(HashMap::new())),
  })
}

fn find_send_ui_message_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Gw\Pref\PrApi.cpp"), Some(r"clampedValue == clientValue"))?;

  let address = scanner.scan_to_function_end(address)?;

  let pattern = pattern![
    0xe8, ?, ?, ?, ?, // CALL send_ui_message_function
    0x83, 0xc4,       // ADD ESP, ...
  ];
  let address = scanner.scan_back_from(&pattern, address)?;
  let send_ui_message_function = scanner.follow_call(address)?;
  log::debug!("Scanned send_ui_message_function: 0x{:08X}", send_ui_message_function);

  Ok(send_ui_message_function)
}

fn find_send_frame_ui_message_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Engine\Frame\FrRelation.cpp"), Some(r"testNode"))?;

  let pattern = pattern![
    0x6a, 0x00,          // PUSH 0x0
    0x6a, 0x01,          // PUSH 0x1
    0x6a, 0x02,          // PUSH 0x2
    0x8d, ?, ?, ?, ?, ?, // LEA ECX, [... + ...]
    0xe8,                // CALL send_frame_ui_message_function
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len() - 1;
  let send_frame_ui_message_function = scanner.follow_call(address)?;
  log::debug!(
    "Scanned send_frame_ui_message_function: 0x{:08X}",
    send_frame_ui_message_function
  );

  Ok(send_frame_ui_message_function)
}

fn find_destroy_frame_ui_message_id(_scanner: &Scanner) -> Result<usize> {
  Ok(0xb)
}

fn find_current_tooltip_pointer(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Engine\Frame\FrTip.cpp"), Some(r"CMsg::Validate(id)"))?;

  let pattern = pattern![
    0x8b, 0x0d, // MOV ECX, [current_tooltip_pointer]
  ];

  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let current_tooltip_pointer = unsafe { (address as *const usize).read_unaligned() };
  log::debug!("Scanned current_tooltip_pointer: 0x{:08X}", current_tooltip_pointer);

  Ok(current_tooltip_pointer)
}

fn find_async_decode_string_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Engine\Text\TextApi.cpp"), Some(r"resultFunc"))?;

  let async_decode_string_function = scanner.scan_back_to_function_start(address)?;
  log::debug!(
    "Scanned async_decode_string_function: 0x{:08X}",
    async_decode_string_function
  );

  Ok(async_decode_string_function)
}
