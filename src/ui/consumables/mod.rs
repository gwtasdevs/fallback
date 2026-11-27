use std::sync::Arc;

use hudhook::imgui::{self, StyleColor, TreeNodeFlags};

use crate::{
  gw::Gw,
  ui::{Module, common::render},
};

pub struct Consumables {
  gw: Arc<Gw>,
}

impl Module for Consumables {
  fn render(&mut self, ui: &hudhook::imgui::Ui, config: &mut super::config::Config) -> bool {
    if !config.consumables.enabled {
      return false;
    }

    let mut config_changed = false;

    if !config.lock_windows {
      let _bg = ui.push_style_color(StyleColor::WindowBg, [0.0, 0.0, 0.0, 0.0]);

      ui.window("##consumables")
        .collapsible(false)
        .position(config.consumables.position, imgui::Condition::FirstUseEver)
        .resizable(true)
        .size(config.consumables.size, imgui::Condition::FirstUseEver)
        .title_bar(false)
        .build(|| {
          let pos = ui.window_pos();
          if pos != config.consumables.position {
            config.consumables.position = pos;
            config_changed = true;
          }

          let size = ui.window_size();
          if size != config.consumables.size {
            config.consumables.size = size;
            config_changed = true;
          }
        });
    }

    let draw_list = ui.get_background_draw_list();
    let min_bounds = config.consumables.position;
    let max_bounds = [
      min_bounds[0] + config.consumables.size[0],
      min_bounds[1] + config.consumables.size[1],
    ];

    draw_list.with_clip_rect(min_bounds, max_bounds, || {});

    config_changed
  }

  fn render_settings(&mut self, ui: &hudhook::imgui::Ui, config: &mut super::config::Config) -> bool {
    let mut config_changed = false;

    let start_pos = ui.cursor_pos();

    let checkbox_width = render::get_checkbox_width(ui);
    let x_offset = ui.content_region_avail()[0] - checkbox_width;

    if x_offset > 0. {
      ui.set_cursor_pos([start_pos[0] + x_offset, start_pos[1]]);
    }

    if ui.checkbox("##enabled", &mut config.consumables.enabled) {
      config_changed = true;
    }
    if ui.is_item_hovered() {
      ui.tooltip_text(if config.consumables.enabled {
        "Disable consumables"
      } else {
        "Enable consumables"
      });
    }

    ui.set_cursor_pos(start_pos);

    let tree = {
      let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
      ui.tree_node_config("Consumables")
        .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
        .push()
    };

    if let Some(_token) = tree {
      ui.indent();

      ui.text("No settings available.");

      ui.unindent();
    }

    config_changed
  }
}

pub fn init(gw: Arc<Gw>) -> Consumables {
  Consumables { gw }
}
