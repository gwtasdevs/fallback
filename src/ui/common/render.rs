use std::{cell::RefCell, collections::HashSet};

use hudhook::imgui::{self, MouseButton, StyleColor, StyleVar, Ui};
use strum::IntoEnumIterator;

use crate::ui::common::{Key, KeyMode};

#[derive(Default, Clone, Debug)]
pub struct Hotkey {
  pub keys: Vec<Key>,
  pub mode: KeyMode,
  pub ctrl: bool,
  pub alt: bool,
  pub shift: bool,
  pub win: bool,
}

pub fn hotkey_box(ui: &Ui, hotkey: &mut Hotkey) -> bool {
  ui.text(format_hotkey(hotkey));

  ui.same_line();

  if ui.button("Modify") {
    ui.open_popup("modify_hotkey");
  }

  ui.same_line();

  let modes: Vec<_> = KeyMode::iter().collect();
  let mut current_mode_index = modes.iter().position(|t| *t == hotkey.mode).unwrap_or(1);
  if ui.combo("Mode", &mut current_mode_index, &modes, |mode| {
    Into::<&str>::into(mode).into()
  }) {
    hotkey.mode = modes[current_mode_index];
    return true;
  }
  let mode = hotkey.mode;

  let next_hotkey = ui
    .modal_popup_config("modify_hotkey")
    .collapsible(false)
    .movable(false)
    .resizable(false)
    .build(|| {
      thread_local! {
        static CAPTURED_HOTKEY: RefCell<Option<Hotkey>> = const { RefCell::new(None) };
      }

      CAPTURED_HOTKEY.with(|captured| {
        let captured = captured.borrow();

        if let Some(captured) = captured.as_ref() {
          ui.text(format_hotkey(captured));
        } else {
          ui.text("Press key combination...");
        }
      });

      ui.separator();

      if ui.is_key_pressed(imgui::Key::Escape) || (ui.is_mouse_clicked(MouseButton::Left) && !ui.is_window_hovered()) {
        CAPTURED_HOTKEY.with(|captured| {
          let mut captured = captured.borrow_mut();
          *captured = None;
        });
        ui.close_current_popup();
        return None;
      }

      let mut alt = false;
      let mut ctrl = false;
      let mut shift = false;
      let mut win = false;
      let mut keys = vec![];

      if ui.is_key_pressed(imgui::Key::LeftAlt) || ui.is_key_pressed(imgui::Key::RightAlt) {
        alt = true;
      }
      if ui.is_key_pressed(imgui::Key::LeftCtrl) || ui.is_key_pressed(imgui::Key::RightCtrl) {
        ctrl = true;
      }
      if ui.is_key_pressed(imgui::Key::LeftShift) || ui.is_key_pressed(imgui::Key::RightShift) {
        shift = true;
      }
      if ui.is_key_pressed(imgui::Key::LeftSuper) || ui.is_key_pressed(imgui::Key::RightSuper) {
        win = true;
      }

      for key in Key::iter() {
        if ui.is_key_pressed(key.into()) {
          keys.push(key);
        }
      }

      CAPTURED_HOTKEY.with(|captured_hotkey| {
        let mut captured_hotkey = captured_hotkey.borrow_mut();

        if let Some(hotkey) = captured_hotkey.take() {
          if alt || ctrl || shift || win || !keys.is_empty() {
            // update the captured hotkey with any newly pressed keys
            let alt = hotkey.alt || alt;
            let ctrl = hotkey.ctrl || ctrl;
            let shift = hotkey.shift || shift;
            let win = hotkey.win || win;

            let mut keys = keys.into_iter().collect::<HashSet<_>>();
            keys.extend(hotkey.keys);
            let keys = keys.into_iter().collect::<Vec<_>>();

            *captured_hotkey = Some(Hotkey {
              keys,
              mode,
              alt,
              ctrl,
              shift,
              win,
            });

            None
          } else {
            // all keys released, save the hotkey
            ui.close_current_popup();
            Some(hotkey)
          }
        } else {
          if alt || ctrl || shift || win || !keys.is_empty() {
            *captured_hotkey = Some(Hotkey {
              keys,
              mode,
              alt,
              ctrl,
              shift,
              win,
            });
          }

          None
        }
      })
    });

  if let Some(Some(next_hotkey)) = next_hotkey {
    *hotkey = next_hotkey;
    true
  } else {
    false
  }
}

fn format_hotkey(hotkey: &Hotkey) -> String {
  let modifiers = [
    (hotkey.ctrl, "Ctrl"),
    (hotkey.alt, "Alt"),
    (hotkey.shift, "Shift"),
    (hotkey.win, "Win"),
  ]
  .into_iter()
  .filter_map(|(on, text)| if on { Some(text.to_owned()) } else { None })
  .collect::<Vec<_>>();

  let keys = hotkey.keys.iter().map(|key| format!("{:?}", key)).collect::<Vec<_>>();

  [modifiers, keys].concat().join(" + ")
}

pub enum ButtonColor {
  None,
  Green,
  Red,
}

pub fn get_checkbox_width(ui: &Ui) -> f32 {
  let inner_spacing_x = unsafe { ui.style().item_inner_spacing[0] };
  ui.frame_height() + inner_spacing_x
}

pub fn get_icon_button_width(ui: &Ui) -> f32 {
  ui.frame_height() + 4.
}

pub fn icon_button(ui: &Ui, label: impl AsRef<str>, hover_text: impl AsRef<str>, color: ButtonColor) -> bool {
  button(ui, label, hover_text, color, Some([get_icon_button_width(ui), 0.]))
}

pub fn button(
  ui: &Ui,
  label: impl AsRef<str>,
  hover_text: impl AsRef<str>,
  color: ButtonColor,
  size: Option<[f32; 2]>,
) -> bool {
  let _token1 = ui.push_style_var(StyleVar::ButtonTextAlign([0.5, 0.5]));

  let colors = match color {
    ButtonColor::None => None,
    ButtonColor::Green => Some((
      [0.15, 0.70, 0.15, 1.00],
      [0.15, 0.85, 0.15, 1.00],
      [0.15, 0.55, 0.15, 1.00],
    )),
    ButtonColor::Red => Some((
      [0.70, 0.15, 0.15, 1.00],
      [0.85, 0.15, 0.15, 1.00],
      [0.55, 0.15, 0.15, 1.00],
    )),
  };

  let _color_tokens = colors.map(|(color, hover_color, active_color)| {
    [
      ui.push_style_color(StyleColor::Button, color),
      ui.push_style_color(StyleColor::ButtonHovered, hover_color),
      ui.push_style_color(StyleColor::ButtonActive, active_color),
    ]
  });

  let is_open = if let Some(size) = size {
    ui.button_with_size(label, size)
  } else {
    ui.button(label)
  };
  if ui.is_item_hovered() {
    ui.tooltip_text(hover_text);
  }

  is_open
}
