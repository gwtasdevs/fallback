use anyhow::{Context, Result};

use crate::{gw::scan::Scanner, parallel, pattern};

#[expect(unused)]
pub struct Render {
  end_scene_fn: usize,
}

#[expect(unused)]
pub fn init(scanner: &Scanner) -> Result<Render> {
  let end_scene = parallel![|| find_end_scene_function(scanner).context("Could not find end_scene function."),]?;

  Ok(Render {
    end_scene_fn: end_scene,
  })
}

fn find_end_scene_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Engine\Gr\Dx9\Dx9Ddi.cpp"),
    Some(r"Encountered draw command outside of draw batch"),
  )?;
  let address = scanner.scan_back_to_function_start(address)?;

  let pattern = pattern![
    0x83, 0xf8, ?,          // CMP EAX, ...
    0x0f, 0x87, ?, ?, ?, ?, // JA ...
    0xff, 0x24, 0x85,       // JMP [switch_table + EAX*4]
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let switch_table = unsafe { (address as *const *const usize).read_unaligned() };
  let address = unsafe { switch_table.add(0xf).read_unaligned() };

  let pattern = pattern![
    0xe8, ?, ?, ?, ?, // CALL end_scene
  ];
  let address = scanner.scan_from(&pattern, address)?;
  let end_scene_function = scanner.follow_call(address)?;

  log::debug!("Scanned end_scene_function: 0x{:08X}", end_scene_function);
  Ok(end_scene_function)
}
