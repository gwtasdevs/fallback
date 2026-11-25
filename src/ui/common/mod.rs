use std::collections::HashSet;

use hudhook::imgui;

pub mod lua;
pub mod render;
pub mod utils;

pub use utils::*;

use crate::lua_enum;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, strum::EnumIter, strum::FromRepr, strum::IntoStaticStr)]
#[repr(u32)]
pub enum Key {
  Tab = imgui::Key::Tab as u32,
  LeftArrow = imgui::Key::LeftArrow as u32,
  RightArrow = imgui::Key::RightArrow as u32,
  UpArrow = imgui::Key::UpArrow as u32,
  DownArrow = imgui::Key::DownArrow as u32,
  PageUp = imgui::Key::PageUp as u32,
  PageDown = imgui::Key::PageDown as u32,
  Home = imgui::Key::Home as u32,
  End = imgui::Key::End as u32,
  Insert = imgui::Key::Insert as u32,
  Delete = imgui::Key::Delete as u32,
  Backspace = imgui::Key::Backspace as u32,
  Space = imgui::Key::Space as u32,
  Enter = imgui::Key::Enter as u32,
  A = imgui::Key::A as u32,
  B = imgui::Key::B as u32,
  C = imgui::Key::C as u32,
  D = imgui::Key::D as u32,
  E = imgui::Key::E as u32,
  F = imgui::Key::F as u32,
  G = imgui::Key::G as u32,
  H = imgui::Key::H as u32,
  I = imgui::Key::I as u32,
  J = imgui::Key::J as u32,
  K = imgui::Key::K as u32,
  L = imgui::Key::L as u32,
  M = imgui::Key::M as u32,
  N = imgui::Key::N as u32,
  O = imgui::Key::O as u32,
  P = imgui::Key::P as u32,
  Q = imgui::Key::Q as u32,
  R = imgui::Key::R as u32,
  S = imgui::Key::S as u32,
  T = imgui::Key::T as u32,
  U = imgui::Key::U as u32,
  V = imgui::Key::V as u32,
  W = imgui::Key::W as u32,
  X = imgui::Key::X as u32,
  Y = imgui::Key::Y as u32,
  Z = imgui::Key::Z as u32,
  F1 = imgui::Key::F1 as u32,
  F2 = imgui::Key::F2 as u32,
  F3 = imgui::Key::F3 as u32,
  F4 = imgui::Key::F4 as u32,
  F5 = imgui::Key::F5 as u32,
  F6 = imgui::Key::F6 as u32,
  F7 = imgui::Key::F7 as u32,
  F8 = imgui::Key::F8 as u32,
  F9 = imgui::Key::F9 as u32,
  F10 = imgui::Key::F10 as u32,
  F11 = imgui::Key::F11 as u32,
  F12 = imgui::Key::F12 as u32,
  Apostrophe = imgui::Key::Apostrophe as u32,
  Comma = imgui::Key::Comma as u32,
  Minus = imgui::Key::Minus as u32,
  Period = imgui::Key::Period as u32,
  Slash = imgui::Key::Slash as u32,
  Semicolon = imgui::Key::Semicolon as u32,
  Equal = imgui::Key::Equal as u32,
  LeftBracket = imgui::Key::LeftBracket as u32,
  Backslash = imgui::Key::Backslash as u32,
  RightBracket = imgui::Key::RightBracket as u32,
  GraveAccent = imgui::Key::GraveAccent as u32,
  CapsLock = imgui::Key::CapsLock as u32,
  ScrollLock = imgui::Key::ScrollLock as u32,
  NumLock = imgui::Key::NumLock as u32,
  PrintScreen = imgui::Key::PrintScreen as u32,
  Pause = imgui::Key::Pause as u32,
  Keypad0 = imgui::Key::Keypad0 as u32,
  Keypad1 = imgui::Key::Keypad1 as u32,
  Keypad2 = imgui::Key::Keypad2 as u32,
  Keypad3 = imgui::Key::Keypad3 as u32,
  Keypad4 = imgui::Key::Keypad4 as u32,
  Keypad5 = imgui::Key::Keypad5 as u32,
  Keypad6 = imgui::Key::Keypad6 as u32,
  Keypad7 = imgui::Key::Keypad7 as u32,
  Keypad8 = imgui::Key::Keypad8 as u32,
  Keypad9 = imgui::Key::Keypad9 as u32,
  KeypadDecimal = imgui::Key::KeypadDecimal as u32,
  KeypadDivide = imgui::Key::KeypadDivide as u32,
  KeypadMultiply = imgui::Key::KeypadMultiply as u32,
  KeypadSubtract = imgui::Key::KeypadSubtract as u32,
  KeypadAdd = imgui::Key::KeypadAdd as u32,
  KeypadEnter = imgui::Key::KeypadEnter as u32,
  KeypadEqual = imgui::Key::KeypadEqual as u32,
  MouseLeft = imgui::Key::MouseLeft as u32,
  MouseRight = imgui::Key::MouseRight as u32,
  MouseMiddle = imgui::Key::MouseMiddle as u32,
  MouseX1 = imgui::Key::MouseX1 as u32,
  MouseX2 = imgui::Key::MouseX2 as u32,
  MouseWheelX = imgui::Key::MouseWheelX as u32,
  MouseWheelY = imgui::Key::MouseWheelY as u32,
}

impl From<Key> for imgui::Key {
  fn from(value: Key) -> Self {
    unsafe { std::mem::transmute(value) }
  }
}

lua_enum!(Key, u32, "Key");

#[derive(
  Clone, Copy, Debug, Default, PartialEq, strum::EnumIter, strum::EnumString, strum::FromRepr, strum::IntoStaticStr,
)]
#[repr(u32)]
pub enum KeyMode {
  Hold,
  #[default]
  Press,
}

lua_enum!(KeyMode, u32, "KeyMode");

pub struct KeyState {
  pub keys_down: HashSet<imgui::Key>,
  pub keys_pressed: HashSet<imgui::Key>,
}

impl KeyState {
  pub fn new(ui: &imgui::Ui) -> Self {
    Self {
      keys_down: imgui::Key::VARIANTS
        .into_iter()
        .filter(|&key| ui.is_key_down(key))
        .collect(),
      keys_pressed: imgui::Key::VARIANTS
        .into_iter()
        .filter(|&key| ui.is_key_pressed(key))
        .collect(),
    }
  }

  pub fn empty() -> Self {
    Self {
      keys_down: HashSet::new(),
      keys_pressed: HashSet::new(),
    }
  }
}
