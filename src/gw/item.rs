use std::sync::Arc;

use anyhow::{Context, Result};

use crate::{
  gw::{models, scan::Scanner},
  parallel, read_ptr,
};

pub struct Item {
  thread: Arc<crate::gw::thread::Thread>,
  ui: Arc<crate::gw::ui::Ui>,
  player_agent_id_ptr: usize,

  use_item_fn: unsafe extern "cdecl" fn(item_id: u32) -> (),
  equip_item_fn: unsafe extern "cdecl" fn(item_id: u32, agent_id: usize) -> (),
}

impl Item {
  pub fn get_inventory(&self) -> Option<*const models::item::Inventory> {
    read_ptr!(self.thread.get_game_context(), .item.player_inventory)
  }

  pub fn get_gold_on_character(&self) -> u32 {
    read_ptr!(self.thread.get_game_context(), .item.player_inventory.gold_character).unwrap_or(0)
  }

  pub fn get_gold_in_xunlai(&self) -> u32 {
    read_ptr!(self.thread.get_game_context(), .item.player_inventory.gold_xunlai).unwrap_or(0)
  }

  pub fn equip_item(&self, item_id: u32) {
    let agent_id = unsafe { (self.player_agent_id_ptr as *const usize).read_unaligned() };
    if agent_id != 0 {
      let func = self.equip_item_fn;
      self.thread.fire_and_forget(move || unsafe {
        func(item_id, agent_id);
      });
    }
  }

  pub fn use_item(&self, item_id: u32) {
    let func = self.use_item_fn;
    self.thread.fire_and_forget(move || unsafe {
      func(item_id);
    });
  }

  pub fn use_item_by_model_id(&self, model_id: u32) {
    if let Some(item) = self.find_item_by_model_id(model_id)
      && let Some(item_id) = read_ptr!(item, .item_id)
    {
      self.use_item(item_id);
    }
  }

  pub fn get_item_by_id(&self, item_id: u32) -> Option<*const models::item::Item> {
    read_ptr!(self.thread.get_game_context(), .item.items[item_id as usize])
  }

  pub fn get_hovered_item(&self) -> Option<*const models::item::Item> {
    models::ui::get_tooltip_item_id(self.ui.get_current_tooltip()).and_then(|item_id| self.get_item_by_id(item_id))
  }

  pub fn find_item_by_model_id(&self, model_id: u32) -> Option<*const models::item::Item> {
    if let Some(inventory) = self.get_inventory() {
      let storage_items = [
        read_ptr!(inventory, .storage_backpack.items),
        read_ptr!(inventory, .storage_belt_pouch_bag.items),
        read_ptr!(inventory, .storage_bag_1.items),
        read_ptr!(inventory, .storage_bag_2.items),
        read_ptr!(inventory, .storage_equipment_pack.items),
      ];
      for items in storage_items.into_iter().flatten() {
        for item in items.into_iter() {
          if Some(model_id) == read_ptr!(item, .model_id) {
            return Some(item);
          }
        }
      }
    }

    None
  }

  pub fn find_item(&self, model_id: u32, modifiers: Vec<u32>) -> Option<*const models::item::Item> {
    if let Some(inventory) = self.get_inventory() {
      let storage_items = [
        read_ptr!(inventory, .storage_backpack.items),
        read_ptr!(inventory, .storage_belt_pouch_bag.items),
        read_ptr!(inventory, .storage_bag_1.items),
        read_ptr!(inventory, .storage_bag_2.items),
        read_ptr!(inventory, .storage_equipment_pack.items),
      ];
      for items in storage_items.into_iter().flatten() {
        'item_loop: for item in items.into_iter() {
          if item.is_null() {
            continue;
          }
          let item_model_id = unsafe { (&raw const (*item).model_id).read_unaligned() };
          if item_model_id != model_id {
            continue;
          }

          let count = unsafe { (&raw const (*item).modifiers_size).read_unaligned() };
          if count != modifiers.len() {
            continue;
          }

          let mods = unsafe { (&raw const (*item).modifiers).read_unaligned() };
          #[expect(clippy::needless_range_loop)]
          for i in 0..count {
            let modifier: u32 = unsafe { std::mem::transmute(mods.add(i).read_unaligned()) };
            if modifier != modifiers[i] {
              continue 'item_loop;
            }
          }

          return Some(item);
        }
      }
    }

    None
  }
}

pub fn init(
  scanner: &Scanner,
  player_agent_id_ptr: usize,
  thread: Arc<crate::gw::thread::Thread>,
  ui: Arc<crate::gw::ui::Ui>,
) -> Result<Item> {
  let (use_item_fn, equip_item_fn) = parallel![
    || find_use_item_function(scanner).context("Could not find use_item function."),
    || find_equip_item_function(scanner).context("Could not find equip_item function."),
  ]?;

  Ok(Item {
    thread,
    ui,
    player_agent_id_ptr,

    use_item_fn: unsafe { std::mem::transmute(use_item_fn) },
    equip_item_fn: unsafe { std::mem::transmute(equip_item_fn) },
  })
}

fn find_use_item_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion_call(r"P:\Code\Gw\Item\Cli\ItCliApi.cpp", r"itemId")?;
  let use_item_function = scanner.scan_back_to_function_start(address)?;
  log::debug!("Scanned use_item_function: 0x{:08X}", use_item_function);

  Ok(use_item_function)
}

fn find_equip_item_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion_call(r"P:\Code\Gw\Ui\Game\GmItemHelpers.cpp", r"targetAgentId")?;
  let equip_item_function = scanner.scan_back_to_function_start(address)?;
  log::debug!("Scanned equip_item_function: 0x{:08X}", equip_item_function);

  Ok(equip_item_function)
}
