use std::{
  ffi::CString,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use anyhow::Result;
use hudhook::{
  ImguiRenderLoop, RenderContext,
  imgui::{self, FontConfig, FontGlyphRanges, FontSource},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, WPARAM},
  UI::{
    Input::KeyboardAndMouse::GetActiveWindow,
    Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
    WindowsAndMessaging::{
      MB_ICONERROR, MB_OK, MessageBoxA, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
    },
  },
};
use windows_core::{PCSTR, s};

use crate::{
  ui::common::{KeyState, lua::init_lua},
  utils::{self, CONFIG_PATH, IS_VSCODE_INSTALLED},
};

mod common;
mod compass;
pub mod config;
mod info;
mod scripts;

const FA_FONT: &[u8] = include_bytes!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/fonts/Font Awesome 7 Free-Solid-900.otf"
));
static DEFAULT_SCRIPTS: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/default_scripts");

static IMGUI_CAPTURING_MOUSE: AtomicBool = AtomicBool::new(false);
static RIGHT_MOUSE_DOWN: AtomicBool = AtomicBool::new(false);

const WND_PROC_HOOK_SUBCLASS_ID: usize = 17563;

pub trait Module: Send + Sync {
  fn render(&mut self, ui: &imgui::Ui, config: &mut config::Config) -> bool;

  fn render_settings(&mut self, ui: &imgui::Ui, config: &mut config::Config) -> bool;
}

type KeyStateSender = UnboundedSender<KeyState>;
type Modules = Vec<Box<dyn Module>>;

pub struct RenderLoop {
  cancel_token: CancellationToken,
  pub config: Option<config::Config>,

  // The latest KeyState sender.
  pub key_state_sender: Option<KeyStateSender>,
  // Every time the modules are initiated, a new KeyState channel is created.
  // Keep a receiver so that we can always send key states into the latest channel.
  pub key_state_sender_receiver: Option<UnboundedReceiver<KeyStateSender>>,

  // The latest modules.
  pub modules: Option<Modules>,
  // The modules list changes, so use a receiver to keep up-to-date with the latest.
  pub modules_receiver: Option<UnboundedReceiver<Modules>>,
}

impl Default for RenderLoop {
  fn default() -> Self {
    Self {
      cancel_token: CancellationToken::new(),
      config: None,
      key_state_sender: None,
      key_state_sender_receiver: None,
      modules: None,
      modules_receiver: None,
    }
  }
}

impl ImguiRenderLoop for RenderLoop {
  fn initialize(&mut self, ctx: &mut imgui::Context, _render_context: &mut dyn RenderContext) {
    log::debug!("RenderLoop initialize fn called");

    ctx.set_ini_filename(None);

    let size_pixels = 13.;
    let default_font_config = FontConfig {
      oversample_h: 3,
      oversample_v: 3,
      pixel_snap_h: true,
      size_pixels,
      ..FontConfig::default()
    };

    let fa_font_config = FontConfig {
      glyph_offset: [0.5, 1.],
      glyph_ranges: FontGlyphRanges::from_slice(&[0xe000, 0xf8ff, 0]), // Unicode Private Use Area
      oversample_h: 3,
      oversample_v: 3,
      pixel_snap_h: true,
      size_pixels,
      ..Default::default()
    };

    ctx.fonts().add_font(&[
      FontSource::DefaultFontData {
        config: Some(default_font_config),
      },
      FontSource::TtfData {
        data: FA_FONT,
        size_pixels,
        config: Some(fa_font_config),
      },
    ]);

    let style = ctx.style_mut();
    style.indent_spacing = 5.;

    if self.config.is_none() {
      init_panic_hook();
      if let Err(err) = init_logs() {
        log::error!("Failed to initialize logs: {}", err);
      }
      if let Err(err) = init_default_scripts() {
        log::error!("Failed to initialize default scripts: {}", err);
      }

      self.config = Some(config::Config::load());

      let cancel_token = self.cancel_token.clone();
      let (key_state_sender_receiver, modules_receiver) = init_modules(cancel_token).unwrap();
      self.key_state_sender_receiver = Some(key_state_sender_receiver);
      self.modules_receiver = Some(modules_receiver);

      // set wnd_proc hook for custom processing of mouse events
      let _ = unsafe { SetWindowSubclass(GetActiveWindow(), Some(wnd_proc_hook), WND_PROC_HOOK_SUBCLASS_ID, 0) };
      log::debug!("wnd_proc hook initialized");
    }
  }

  fn before_wnd_proc(&self, _hwnd: HWND, umsg: u32, _wparam: WPARAM, _lparam: LPARAM) -> hudhook::BeforeWndProc {
    match umsg {
      WM_RBUTTONDOWN => {
        RIGHT_MOUSE_DOWN.store(true, Ordering::Relaxed);
        hudhook::BeforeWndProc::Break
      }
      WM_RBUTTONUP => {
        RIGHT_MOUSE_DOWN.store(false, Ordering::Relaxed);
        hudhook::BeforeWndProc::Break
      }
      _ => {
        // If the right mouse button is down then the user is panning the camera,
        // so do not send any mouse or key events to imgui.
        if RIGHT_MOUSE_DOWN.load(Ordering::Relaxed) {
          hudhook::BeforeWndProc::Break
        } else {
          hudhook::BeforeWndProc::Continue
        }
      }
    }
  }

  fn before_render(&mut self, ctx: &mut imgui::Context, _render_context: &mut dyn RenderContext) {
    let io = ctx.io();

    if io.want_capture_mouse {
      IMGUI_CAPTURING_MOUSE.store(true, Ordering::Relaxed);
      return;
    }

    if let Some(config) = self.config.as_ref() {
      let [x, y] = io.mouse_pos;

      let [center_x, center_y] = config.compass.position;
      let radius = config.compass.size * 0.5;
      let min_bounds = [center_x - radius, center_y - radius];
      let max_bounds = [center_x + radius, center_y + radius];
      if x >= min_bounds[0] && x <= max_bounds[0] && y >= min_bounds[1] && y <= max_bounds[1] {
        IMGUI_CAPTURING_MOUSE.store(true, Ordering::Relaxed);
        return;
      }
    }

    IMGUI_CAPTURING_MOUSE.store(false, Ordering::Relaxed);
  }

  fn render(&mut self, ui: &mut imgui::Ui) {
    if let Some(config) = self.config.as_mut() {
      let mut config_changed = false;

      // update self.key_state_sender
      if let Some(receiver) = self.key_state_sender_receiver.as_mut() {
        let mut key_state_sender = None;
        while let Ok(k) = receiver.try_recv() {
          key_state_sender = Some(k);
        }

        if key_state_sender.is_some() {
          self.key_state_sender = key_state_sender;
        }
      }

      // update self.modules
      if let Some(receiver) = self.modules_receiver.as_mut() {
        let mut modules = None;

        while let Ok(m) = receiver.try_recv() {
          modules = Some(m);
        }

        if modules.is_some() {
          self.modules = modules;
        }
      }

      if let Some(key_state_sender) = self.key_state_sender.as_ref()
        && let Err(err) = key_state_sender.send(KeyState::new(ui))
      {
        log::error!("Failed to send keystroke: {}", err);
      }

      if self.modules.as_ref().is_some_and(|modules| modules.is_empty()) {
        // do not even render the main window if we have loaded but there are no modules (happens if in PvP map).
        return;
      }

      ui.window("Fallback!")
        .position(config.window.position, imgui::Condition::FirstUseEver)
        .size(config.window.size, imgui::Condition::FirstUseEver)
        .build(|| {
          let pos = ui.window_pos();
          if pos != config.window.position {
            config.window.position = pos;
            config_changed = true;
          }

          let size = ui.window_size();
          if size != config.window.size {
            config.window.size = size;
            config_changed = true;
          }

          config_changed = config_changed && !ui.is_mouse_down(imgui::MouseButton::Left);

          if let Some(modules) = self.modules.as_mut() {
            let _render_settings_token = ui.push_id("render_settings");
            for (i, module) in modules.iter_mut().enumerate() {
              let _token = ui.push_id_usize(i);
              if module.render_settings(ui, config) {
                config_changed = true;
              }
            }

            if ui.checkbox("Lock windows", &mut config.lock_windows) {
              config_changed = true;
            }
          } else {
            ui.text("Loading...");
          }

          if ui.button("Exit Fallback!") {
            exit(self.cancel_token.clone());
          }
        });

      if let Some(modules) = self.modules.as_mut() {
        let _render_token = ui.push_id("render");
        for (i, module) in modules.iter_mut().enumerate() {
          let _token = ui.push_id_usize(i);
          if module.render(ui, config) {
            config_changed = true;
          }
        }
      }

      if config_changed {
        config.save();
      }
    }
  }
}

fn init_panic_hook() {
  let old_panic_hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(move |panic_info| {
    let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
      *s
    } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
      s.as_str()
    } else {
      "An unexpected panic occurred."
    };

    let location = panic_info
      .location()
      .map(|l| format!(" at {}:{}", l.file(), l.line()))
      .unwrap_or_default();

    let msg = format!("Panic: {}\n{}", msg, location);
    log::error!("{}", msg);
    let msg = CString::new(msg).unwrap_or_default();

    unsafe {
      MessageBoxA(
        None,
        PCSTR::from_raw(msg.as_ptr() as *const u8),
        s!("Fallback panicked!"),
        MB_OK | MB_ICONERROR,
      );
    };
    old_panic_hook(panic_info);
  }));
}

fn init_default_scripts() -> Result<()> {
  for dir in DEFAULT_SCRIPTS.dirs() {
    if CONFIG_PATH.join(dir.path()).exists() {
      return Ok(());
    }
  }

  log::info!("No scripts found, initializing with the defaults.");
  DEFAULT_SCRIPTS.extract(&*CONFIG_PATH)?;
  Ok(())
}

fn init_logs() -> Result<()> {
  #[cfg(debug_assertions)]
  {
    unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "1") };
  }

  // TODO: label log files by date, erase old ones
  let log_path = utils::CONFIG_PATH.join("logs");
  std::fs::create_dir_all(&log_path)?;
  let log_file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(log_path.join("log.txt"))?;

  let log_level = if cfg!(debug_assertions) {
    log::LevelFilter::Debug
  } else {
    log::LevelFilter::Info
  };
  simplelog::WriteLogger::init(log_level, simplelog::Config::default(), log_file)?;
  log::debug!("Logging initialized.");

  let _ = *IS_VSCODE_INSTALLED;

  Ok(())
}

fn init_modules(
  cancel_token: CancellationToken,
) -> Result<(UnboundedReceiver<KeyStateSender>, UnboundedReceiver<Modules>)> {
  let (key_state_sender_sender, key_state_sender_receiver) = tokio::sync::mpsc::unbounded_channel();
  let (modules_sender, modules_receiver) = tokio::sync::mpsc::unbounded_channel();

  std::thread::spawn(move || -> Result<()> {
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let handle = tokio_runtime.handle().clone();
    tokio_runtime.block_on(async move {
      if let Err(err) = sync_modules(key_state_sender_sender, modules_sender, handle, cancel_token).await {
        log::error!("Failed creating modules: {}.", err);
      }
    });

    Ok(())
  });

  Ok((key_state_sender_receiver, modules_receiver))
}

async fn sync_modules(
  key_state_sender_sender: UnboundedSender<KeyStateSender>,
  modules_sender: UnboundedSender<Modules>,
  handle: tokio::runtime::Handle,
  cancel_token: CancellationToken,
) -> Result<()> {
  let gw = Arc::new(crate::gw::init()?);
  let gw_ref = gw.clone();
  log::debug!("GW API initialized.");

  let lua = Arc::new(init_lua(gw.clone()));
  log::debug!("Lua initialized.");

  let cancel_token_ref = cancel_token.clone();

  let create_modules = move || -> Result<Modules> {
    let (key_state_sender, key_state_receiver) = tokio::sync::mpsc::unbounded_channel();
    key_state_sender_sender.send(key_state_sender)?;

    Ok(vec![
      Box::new(compass::init(gw.clone(), lua.clone(), cancel_token.child_token())?),
      Box::new(info::init(gw.clone(), cancel_token.child_token())?),
      Box::new(scripts::init(
        gw.clone(),
        lua.clone(),
        key_state_receiver,
        handle.clone(),
        cancel_token.child_token(),
      )?),
    ])
  };

  modules_sender.send(create_modules()?)?;
  log::debug!("UI Modules sync initialized");

  // disable all modules when in a pvp map
  let is_last_map_pvp = AtomicBool::new(false);
  let gw = gw_ref.clone();
  let map_loaded_hook_id = gw_ref.map.register_map_loaded_hook(move |_| {
    if let Some(map_id) = gw.map.get_map_id() {
      let is_pvp_map = gw.map.get_is_pvp_map(map_id);
      let was_pvp_map = is_last_map_pvp.swap(is_pvp_map, Ordering::Relaxed);

      if was_pvp_map && !is_pvp_map {
        log::debug!("In a non-PvP map, re-initializing all modules.");
        let _ = modules_sender.send(create_modules().unwrap());
      } else if !was_pvp_map && is_pvp_map {
        log::debug!("In a PvP map, all dropping modules.");
        let _ = modules_sender.send(vec![]);
      }
    }
  });

  scopeguard::defer! {
    gw_ref.ui.deregister_send_ui_message_hook(&map_loaded_hook_id);
    log::debug!("UI modules have stopped syncing.");
  }

  cancel_token_ref.cancelled().await;

  Ok(())
}

fn exit(cancel_token: CancellationToken) {
  log::debug!("Exiting fallback");

  let _ = unsafe { RemoveWindowSubclass(GetActiveWindow(), Some(wnd_proc_hook), WND_PROC_HOOK_SUBCLASS_ID) };
  cancel_token.cancel();
  hudhook::eject();
}

unsafe extern "system" fn wnd_proc_hook(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
  _id_subclass: usize,
  _ref_data: usize,
) -> LRESULT {
  // Block left mouse clicks if the imgui window is affected by them, unless the right mouse button is down.
  // This is to allow the user to walk by holding right+left mouse buttons, even whilst hovered over the imgui
  // window.
  if IMGUI_CAPTURING_MOUSE.load(Ordering::Relaxed) && !RIGHT_MOUSE_DOWN.load(Ordering::Relaxed) {
    match msg {
      WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP => {
        return LRESULT(1);
      }
      _ => {}
    }
  }

  unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
