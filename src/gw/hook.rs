use std::{
  collections::HashMap,
  sync::{Arc, RwLock, mpsc},
};

use anyhow::Result;
use ilhook::x86::{CallbackOption, ClosureHookPoint, HookFlags, Registers, hook_closure_jmp_back};
use uuid::Uuid;

struct HookPoint<'a> {
  _inner: ClosureHookPoint<'a>,
}

unsafe impl<'a> Send for HookPoint<'a> {}
unsafe impl<'a> Sync for HookPoint<'a> {}

type HookFn = Box<dyn Fn(&Registers) + Send + Sync>;
type OneShotFn = Box<dyn FnOnce(&Registers) + Send>;

pub struct Hook<'a> {
  _hook_point: Option<HookPoint<'a>>,
  hooks: Arc<RwLock<HashMap<Uuid, HookFn>>>,
  one_shots: mpsc::Sender<OneShotFn>,
}

impl Hook<'_> {
  pub fn new(address: usize) -> Result<Self> {
    let hooks = Arc::new(RwLock::new(HashMap::<Uuid, HookFn>::new()));
    let hooks_ref = Arc::clone(&hooks);
    let (one_shots, rx) = mpsc::channel::<OneShotFn>();

    let hook_point = unsafe {
      hook_closure_jmp_back(
        address,
        move |reg| {
          let reg = &*reg;

          if let Ok(hooks) = hooks_ref.read() {
            for func in hooks.values() {
              func(reg);
            }
          } else {
            log::error!("hooks lock poisoned.");
          }

          while let Ok(func) = rx.try_recv() {
            func(reg);
          }
        },
        CallbackOption::None,
        HookFlags::empty(),
      )?
    };

    Ok(Hook {
      _hook_point: Some(HookPoint { _inner: hook_point }),
      hooks,
      one_shots,
    })
  }

  pub async fn execute_once<F, R>(&self, func: F) -> R
  where
    F: FnOnce(&Registers) -> R + Send + 'static,
    R: Send + 'static,
  {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let _ = self.one_shots.send(Box::new(move |reg| {
      let _ = tx.send(func(reg));
    }));

    rx.await.unwrap()
  }

  pub fn fire_and_forget<F>(&self, func: F)
  where
    F: FnOnce(&Registers) + Send + 'static,
  {
    let _ = self.one_shots.send(Box::new(move |reg| {
      func(reg);
    }));
  }

  pub fn register_hook<F: Fn(&Registers) + Send + Sync + 'static>(&self, func: F) -> Uuid {
    let id = Uuid::new_v4();
    if let Ok(mut hooks) = self.hooks.write() {
      hooks.insert(id, Box::new(func));
    }

    id
  }

  pub fn deregister_hook(&self, id: &Uuid) -> bool {
    if let Ok(mut hooks) = self.hooks.write() {
      hooks.remove(id).is_some()
    } else {
      log::error!("hooks lock poisoned.");
      false
    }
  }
}
