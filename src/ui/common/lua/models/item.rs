use crate::{gw::models, lua_struct};

lua_struct! {
  struct Item {
    item_id: u32,
  }
}

impl Item {
  pub fn from_raw(item: *const models::item::Item) -> Self {
    unsafe {
      let item_id = (&raw const (*item).item_id).read_unaligned();

      Self { item_id }
    }
  }
}
