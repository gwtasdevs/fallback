use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use hudhook::imgui::{self, TreeNodeFlags};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{
  gw::{
    Gw,
    models::{self, agent::into_agent},
  },
  ui::{
    Module,
    common::{
      render::{self, ButtonColor},
      utils,
    },
    config::Config,
  },
};

pub struct Info {
  gw: Arc<Gw>,
  hovered_item: Arc<ArcSwap<Option<Item>>>,
  targeted_agent: Arc<ArcSwap<Option<Agent>>>,
  _cancel_guard: DropGuard,
}

struct Agent {
  name: Option<String>,
  model_id: Option<u16>,
  x: f32,
  y: f32,
}

struct Item {
  name: Option<String>,
  complete_name: Option<String>,
  model_id: u32,
  modifiers: Vec<u32>,
  unique_id: String,
}

impl Info {
  fn render_info_window(&mut self, ui: &imgui::Ui) {
    if let Some(player) = self.gw.agent.get_player() {
      let tree = {
        let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
        ui.tree_node_config("Player")
          .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
          .push()
      };

      if let Some(_token) = tree {
        ui.indent();

        let x = unsafe { (&raw const (*player).base.x).read_unaligned() };
        let y = unsafe { (&raw const (*player).base.y).read_unaligned() };

        self.render_with_copy(ui, "Position", &format!("{:.0}, {:.0}", x, y));

        ui.unindent();
      }
    }

    if let Some(target) = self.targeted_agent.load().as_ref() {
      let tree = {
        let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
        ui.tree_node_config("Target")
          .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
          .push()
      };

      if let Some(_token) = tree {
        ui.indent();

        self.render_with_copy(ui, "Name", target.name.as_deref().unwrap_or(""));
        self.render_with_copy(ui, "Position", &format!("{:.0}, {:.0}", target.x, target.y));

        if let Some(model_id) = target.model_id {
          self.render_with_copy(ui, "Model ID", &model_id.to_string());
        }

        ui.unindent();
      }
    }

    if let Some(item) = self.hovered_item.load().as_ref() {
      let tree = {
        let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
        ui.tree_node_config("Hovered Item")
          .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
          .push()
      };

      if let Some(_token) = tree {
        ui.indent();

        self.render_with_copy(ui, "Full Name", item.complete_name.as_deref().unwrap_or(""));
        self.render_with_copy(ui, "Name", item.name.as_deref().unwrap_or(""));
        self.render_with_copy(ui, "Model ID", &item.model_id.to_string());
        self.render_with_copy(ui, "Unique ID", &item.unique_id);

        let mods = format!(
          "[{}]",
          item
            .modifiers
            .iter()
            .map(|m| format!("{:08X}", m))
            .collect::<Vec<_>>()
            .join(", ")
        );
        self.render_with_copy(ui, "Mods", &mods);

        ui.unindent();
      }
    }
  }

  fn render_with_copy(&mut self, ui: &imgui::Ui, label: &str, value: &str) {
    ui.text_wrapped(format!("{}: {}", label, value));
    ui.same_line();
    if render::icon_button(
      ui,
      format!("\u{f0c5}##{}", label),
      format!("Copy {}", label),
      ButtonColor::None,
    ) && let Ok(mut clipboard) = arboard::Clipboard::new()
    {
      let _ = clipboard.set_text(value);
    }
  }
}

impl Module for Info {
  fn render(&mut self, ui: &imgui::Ui, config: &mut Config) -> bool {
    if !config.info.enabled {
      return false;
    }

    let mut config_changed = false;

    ui.window("Info")
      .collapsed(config.info.collapsed, imgui::Condition::FirstUseEver)
      .position(config.info.position, imgui::Condition::FirstUseEver)
      .size(config.info.size, imgui::Condition::FirstUseEver)
      .build(|| {
        let pos = ui.window_pos();
        if pos != config.info.position && !ui.is_mouse_down(imgui::MouseButton::Left) {
          config.info.position = pos;
          config_changed = true;
        }

        let size = ui.window_size();
        if size != config.info.size && !ui.is_mouse_down(imgui::MouseButton::Left) {
          config.info.size = size;
          config_changed = true;
        }

        self.render_info_window(ui);
      });

    config_changed
  }

  fn render_settings(&mut self, ui: &imgui::Ui, config: &mut Config) -> bool {
    let mut config_changed = false;

    let start_pos = ui.cursor_pos();

    let checkbox_width = render::get_checkbox_width(ui);
    let x_offset = ui.content_region_avail()[0] - checkbox_width;

    if x_offset > 0. {
      ui.set_cursor_pos([start_pos[0] + x_offset, start_pos[1]]);
    }

    if ui.checkbox("##enabled", &mut config.info.enabled) {
      config_changed = true;
    }
    if ui.is_item_hovered() {
      ui.tooltip_text(if config.info.enabled {
        "Disable info window"
      } else {
        "Enable info window"
      });
    }

    ui.set_cursor_pos(start_pos);

    let _tree = {
      let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
      ui.tree_node_config("Info")
        .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::BULLET)
        .push()
    };

    config_changed
  }
}

pub fn init(gw: Arc<Gw>, cancel_token: CancellationToken) -> Result<Info> {
  let hovered_item = Arc::new(ArcSwap::from_pointee(None));
  let targeted_agent = Arc::new(ArcSwap::from_pointee(None));

  let _cancel_guard = cancel_token.clone().drop_guard();
  let hovered_item_ref = hovered_item.clone();
  let targeted_agent_ref = targeted_agent.clone();
  let gw_ref = gw.clone();
  tokio::spawn(async move {
    let mut set = tokio::task::JoinSet::new();

    set.spawn(async move {
      let mut last_hovered_item_ptr = 0;
      let mut last_targeted_agent_ptr = 0;

      loop {
        if let Some(item) = gw.item.get_hovered_item() {
          let item = item as usize;
          if item != last_hovered_item_ptr {
            last_hovered_item_ptr = item;

            let cached_item = cache_item(item, gw.clone()).await;
            hovered_item_ref.store(Arc::new(Some(cached_item)));
          }
        }

        if let Some(base_agent) = gw.agent.get_target() {
          let base_agent = base_agent as usize;
          if base_agent != last_targeted_agent_ptr {
            last_targeted_agent_ptr = base_agent;

            let cached_agent = cache_agent(base_agent, gw.clone()).await;
            targeted_agent_ref.store(Arc::new(Some(cached_agent)));
          }
        } else {
          last_targeted_agent_ptr = 0;
          targeted_agent_ref.store(Arc::new(None));
        }

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
      }
    });

    cancel_token.cancelled().await;
    set.shutdown().await;
  });

  Ok(Info {
    gw: gw_ref,
    hovered_item,
    targeted_agent,
    _cancel_guard,
  })
}

async fn cache_agent(base_agent_ptr: usize, gw: Arc<Gw>) -> Agent {
  let base = base_agent_ptr as *const models::agent::BaseAgent;

  let agent_id = unsafe { (&raw const (*base).agent_id).read_unaligned() };
  let x = unsafe { (&raw const (*base).x).read_unaligned() };
  let y = unsafe { (&raw const (*base).y).read_unaligned() };

  let model_id = into_agent(base)
    .ok()
    .map(|agent| unsafe { (&raw const (*agent).model_id).read_unaligned() });

  let name = gw.agent.get_agent_name(agent_id).await;

  Agent { name, model_id, x, y }
}

async fn cache_item(item_ptr: usize, gw: Arc<Gw>) -> Item {
  let item = item_ptr as *const models::item::Item;
  let mut modifiers = vec![];
  let mods = unsafe { (&raw const (*item).modifiers).read_unaligned() };
  let count = unsafe { (&raw const (*item).modifiers_size).read_unaligned() };
  for i in 0..count {
    let modifier = unsafe { mods.add(i).read_unaligned() };
    modifiers.push(unsafe { std::mem::transmute(modifier) });
  }

  let model_id = unsafe { (&raw const (*item).model_id).read_unaligned() };

  let unique_id = utils::encode_unique_item_id(model_id, modifiers.clone());

  let encoded_name = unsafe { (&raw const (*item).encoded_name).read_unaligned() };
  let encoded_complete_name = unsafe { (&raw const (*item).encoded_complete_name).read_unaligned() };

  let encoded_name = gw.ui.try_clone_encoded_string(encoded_name);
  let encoded_complete_name = gw.ui.try_clone_encoded_string(encoded_complete_name);

  let name = if let Some(encoded_name) = encoded_name {
    gw.ui.decode_string(encoded_name).await
  } else {
    None
  };
  let complete_name = if let Some(encoded_complete_name) = encoded_complete_name {
    gw.ui.decode_string(encoded_complete_name).await
  } else {
    None
  };

  Item {
    name,
    complete_name,
    model_id,
    modifiers,
    unique_id,
  }
}
