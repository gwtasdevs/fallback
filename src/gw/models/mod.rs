use std::ptr::null;

pub mod agent;
pub mod camera;
pub mod chat;
pub mod contexts;
pub mod item;
pub mod map;
pub mod player;
pub mod skill;
pub mod ui;
pub mod ui_message;

derive_alias! {
  #[derive(Enum!)] = #[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    strum::AsRefStr,
    strum::EnumIter,
    strum::FromRepr,
    strum::IntoStaticStr,
  )];
}

#[macro_export]
macro_rules! read_ptr {
  ($base:expr, . $field:ident) => {{
    let ptr: *const _ = $base;
    if ptr.is_null() {
      None
    } else {
      unsafe {
        let ptr = &raw const (*ptr).$field;
        Some(ptr.read_unaligned())
      }
    }
  }};

  ($base:expr, . $field:ident [ $index:expr ]) => {{
    if let Some(arr) = read_ptr!($base, . $field) {
      let arr: $crate::gw::models::Array<_> = arr;
      arr.get($index)
    } else {
      None
    }
  }};

  ($base:expr, . $field:ident [ $index:expr ] . $($rest:tt)+) => {{
    if let Some(arr) = read_ptr!($base, . $field) {
      let arr: $crate::gw::models::Array<_> = arr;
      let next_ptr = arr.get_ptr($index);
      read_ptr!(next_ptr, . $($rest)+)
    } else {
      None
    }
  }};

  ($base:expr, . $field:ident . $($rest:tt)+) => {
    {
      if let Some(next_ptr) = read_ptr!($base, . $field) {
        read_ptr!(next_ptr, . $($rest)+)
      } else {
        None
      }
    }
  };
}

#[repr(C, packed)]
pub struct Position {
  /* +000 */ pub x: f32,
  /* +004 */ pub y: f32,
  /* +008 */ pub z_plane: u32,
  /* +00c */
}

#[repr(C, packed)]
pub struct Array<T> {
  /* +000 */ buffer: *const T,
  /* +004 */ pub capacity: u32,
  /* +008 */ pub size: u32,
  /* +00c */ param: u32,
  /* +010 */
}

impl<T> Array<T> {
  pub unsafe fn from_address(address: usize) -> Self {
    unsafe { (address as *const Self).read_unaligned() }
  }

  pub fn is_null(&self) -> bool {
    self.buffer.is_null()
  }

  pub fn get_ptr(&self, index: usize) -> *const T {
    if self.is_null() || index >= self.size as usize {
      return null();
    }
    unsafe { &raw const *self.buffer.add(index) }
  }

  pub fn get(&self, index: usize) -> Option<T> {
    if self.is_null() || index >= self.size as usize {
      return None;
    }
    let target = unsafe { self.buffer.add(index).read_unaligned() };

    Some(target)
  }

  pub fn into_iter(self) -> ArrayIntoIterator<T> {
    ArrayIntoIterator {
      buffer: self.buffer,
      size: self.size as usize,
      current_index: 0,
    }
  }

  pub fn iter(&self) -> ArrayIterator<T> {
    ArrayIterator {
      buffer: self.buffer,
      size: self.size as usize,
      current_index: 0,
    }
  }
}

pub struct ArrayIntoIterator<T> {
  buffer: *const T,
  size: usize,
  current_index: usize,
}

impl<T> Iterator for ArrayIntoIterator<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if self.buffer.is_null() || self.current_index >= self.size {
      return None;
    }

    unsafe {
      let element = self.buffer.add(self.current_index).read_unaligned();
      self.current_index += 1;
      Some(element)
    }
  }
}

pub struct ArrayIterator<T> {
  buffer: *const T,
  size: usize,
  current_index: usize,
}

impl<T> Iterator for ArrayIterator<T> {
  type Item = *const T;

  fn next(&mut self) -> Option<Self::Item> {
    if self.buffer.is_null() || self.current_index >= self.size {
      return None;
    }

    unsafe {
      let element = &raw const *self.buffer.add(self.current_index);
      self.current_index += 1;
      Some(element)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_struct_sizes() {
    assert_eq!(std::mem::size_of::<Position>(), 0xc);
    assert_eq!(std::mem::size_of::<Array<u32>>(), 0x10);
  }

  #[test]
  fn test_access() {
    let buffer = [0usize, 100, 200];

    let array_memory = [(&buffer) as *const usize as usize, 4, 3, 0];

    let array = unsafe { Array::<usize>::from_address(&array_memory as *const usize as usize) };

    assert_eq!({ array.capacity }, 4);
    assert_eq!({ array.size }, 3);
    assert_eq!({ array.param }, 0);
    assert_eq!(array.get(0), Some(0));
    assert_eq!(array.get(1), Some(100));
    assert_eq!(array.get(2), Some(200));
  }
}
