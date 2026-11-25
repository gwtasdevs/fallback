use anyhow::{Context, Result};
use uuid::Uuid;

use crate::{
  gw::{hook::Hook, models::contexts::GameContext, scan::Scanner},
  parallel, pattern,
};

pub struct Thread {
  base_ptr: usize,
  game_thread_hook: Hook<'static>,
}

impl Thread {
  pub fn get_game_context(&self) -> *const GameContext {
    let base_pointer = self.base_ptr as *const *const *const GameContext;

    unsafe {
      let base_context = base_pointer.read_unaligned();
      if base_context.is_null() {
        return std::ptr::null();
      }

      base_context.add(6).read_unaligned()
    }
  }

  pub async fn execute<F, R>(&self, func: F) -> R
  where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
  {
    self.game_thread_hook.execute_once(|_reg| func()).await
  }

  pub fn fire_and_forget<F>(&self, func: F)
  where
    F: FnOnce() + Send + 'static,
  {
    self.game_thread_hook.fire_and_forget(|_reg| func());
  }

  pub fn register_on_tick_hook<F: Fn() + Send + Sync + 'static>(&self, func: F) -> Uuid {
    self.game_thread_hook.register_hook(move |_reg| func())
  }

  pub fn deregister_on_tick_hook(&self, id: &Uuid) -> bool {
    self.game_thread_hook.deregister_hook(id)
  }
}

pub fn init(scanner: &Scanner) -> Result<Thread> {
  let (base_ptr, game_thread_entry_fn) = parallel![
    || find_base_pointer(scanner).context("Count not find base pointer."),
    || find_game_thread_entry_function(scanner).context("Could not find game_thread_entry function."),
  ]?;

  let game_thread_hook = Hook::new(game_thread_entry_fn)?;

  Ok(Thread {
    base_ptr,
    game_thread_hook,
  })
}

fn find_base_pointer(scanner: &Scanner) -> Result<usize> {
  #[rustfmt::skip]
  let pattern = pattern![
    0x50,       // PUSH EAX
    0x6a, 0x0f, // PUSH 0xf
    0x6a, 0x00, // PUSH 0x0
    0xff, 0x35, // PUSH [base_ptr]
  ];

  let address = scanner.scan(&pattern)? + pattern.len();
  let base_ptr = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned base_ptr: 0x{:08X}", base_ptr);
  Ok(base_ptr)
}

fn find_game_thread_entry_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Engine\Frame\FrApi.cpp"), Some(r"!s_bufferBits"))?;
  let pattern = pattern![
    0xe8, ?, ?, ?, ?,        // CALL ...
    0x6a, 0x00,              // PUSH 0x0
    0x68, ?, ?, ?, ?,        // PUSH ...
    0x6a, 0x01,              // PUSH 0x1
    0xe8, ?, ?, ?, ?,        // CALL ...
    0x6a, 0x00,              // PUSH 0x0
    0x68,                    // PUSH game_thread_entry_fn
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let game_thread_entry_fn = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned game_thread_entry_fn: 0x{:08X}", game_thread_entry_fn);
  Ok(game_thread_entry_fn)
}
