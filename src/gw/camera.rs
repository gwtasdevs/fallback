use std::f32::consts::PI;

use anyhow::{Context, Result};

use crate::{
  gw::{models, scan::Scanner},
  parallel, pattern,
};

pub struct Camera {
  pub camera_ptr: usize,
}

impl Camera {
  pub fn get_yaw(&self) -> f32 {
    let camera = self.camera_ptr as *const models::camera::Camera;
    let x = unsafe { (&raw const (*camera).position_x).read_unaligned() };
    let y = unsafe { (&raw const (*camera).position_y).read_unaligned() };
    let look_at_x = unsafe { (&raw const (*camera).look_at_target_x).read_unaligned() };
    let look_at_y = unsafe { (&raw const (*camera).look_at_target_y).read_unaligned() };

    let cur_tan = (y - look_at_y).atan2(x - look_at_x);

    if cur_tan >= 0. { cur_tan - PI } else { cur_tan + PI }
  }
}

pub fn init(scanner: &Scanner) -> Result<Camera> {
  let camera_ptr = parallel![|| find_camera_pointer(scanner).context("Could not find camera pointer."),]?;

  Ok(Camera { camera_ptr })
}

fn find_camera_pointer(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Gw\Ui\Game\GmCam.cpp"), Some(r"fov"))?;

  #[rustfmt::skip]
  let pattern = pattern![
    0xd9, 0xee, // FLDZ
    0xb9,       // MOV ECX, camera_pointer
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let camera_pointer = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned camera_pointer: 0x{:08X}", camera_pointer);
  Ok(camera_pointer)
}
