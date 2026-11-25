use std::{ptr::null, sync::Arc};

use anyhow::{Context, Result};

use crate::{
  gw::{
    models::{
      self, Array,
      contexts::{PathingMap, PathingPoint, PathingTrapezoid},
      map::{AreaFlags, AreaInfo, InstanceInfo, InstanceType},
      ui_message::MapLoadedUiMessage,
    },
    scan::Scanner,
  },
  parallel, pattern, read_ptr,
};

pub struct Map {
  thread: Arc<crate::gw::thread::Thread>,
  ui: Arc<crate::gw::ui::Ui>,

  find_path_fn: unsafe extern "cdecl" fn(
    start_point: *const PathingPoint,
    end_point: *const PathingPoint,
    range: f32,
    result_max_capacity: u32,
    out_result_count: *mut usize,
    out_result_array: *mut PathingPoint,
  ) -> (),
  area_info_ptr: usize,
  instance_info_ptr: usize,
  map_loaded_ui_message_id: usize,
  map_travel_ui_message_id: usize,
  current_region_id_ptr: usize,
}

impl Map {
  pub async fn find_path(&self, start: (f32, f32), end: (f32, f32)) -> Option<Vec<PathingPoint>> {
    if let Some(pathing_maps) = self.get_pathing_maps() {
      let mut start_point = None;
      let mut end_point = None;

      for map in pathing_maps.iter() {
        if map.is_null() {
          continue;
        }

        let trapezoid_count = unsafe { (&raw const (*map).trapezoid_count).read_unaligned() } as usize;
        for i in 0..trapezoid_count {
          let trapezoids = unsafe { (&raw const (*map).trapezoids).read_unaligned() };
          let trapezoid = unsafe { trapezoids.add(i) };
          if trapezoid.is_null() {
            continue;
          }

          if Self::point_in_trapezoid(start.0, start.1, trapezoid) {
            start_point = Some(PathingPoint {
              x: start.0,
              y: start.1,
              z_plane: 0,
              trapezoid,
            });
          }
          if Self::point_in_trapezoid(end.0, end.1, trapezoid) {
            end_point = Some(PathingPoint {
              x: end.0,
              y: end.1,
              z_plane: 0,
              trapezoid,
            });
          }

          if start_point.is_some() && end_point.is_some() {
            break;
          }
        }

        if start_point.is_some() && end_point.is_some() {
          break;
        }
      }

      if let Some(start_point) = start_point
        && let Some(end_point) = end_point
      {
        let func = self.find_path_fn;
        let start_point_ptr = &raw const start_point as usize;
        let end_point_ptr = &raw const end_point as usize;
        self
          .thread
          .execute(move || {
            let mut count = 0;
            let mut result: [PathingPoint; 10] = [PathingPoint {
              x: 0.,
              y: 0.,
              z_plane: 0,
              trapezoid: null(),
            }; 10];

            unsafe {
              func(
                start_point_ptr as *const PathingPoint,
                end_point_ptr as *const PathingPoint,
                10000.,
                10,
                &raw mut count,
                result.as_mut_ptr(),
              );
            }

            let mut result = Vec::from(result);
            result.truncate(count);

            Some(result)
          })
          .await
      } else {
        None
      }
    } else {
      None
    }
  }

  fn point_in_trapezoid(x: f32, y: f32, trapezoid: *const PathingTrapezoid) -> bool {
    if trapezoid.is_null() {
      return false;
    }

    let xtl = unsafe { (&raw const (*trapezoid).xtl).read_unaligned() };
    let xbl = unsafe { (&raw const (*trapezoid).xbl).read_unaligned() };
    let xtr = unsafe { (&raw const (*trapezoid).xtr).read_unaligned() };
    let xbr = unsafe { (&raw const (*trapezoid).xbr).read_unaligned() };
    let yt = unsafe { (&raw const (*trapezoid).yt).read_unaligned() };
    let yb = unsafe { (&raw const (*trapezoid).yb).read_unaligned() };

    if yt < y || yb > y {
      return false;
    }
    if xtl > x && xbl > x {
      return false;
    }
    if xtr < x && xbr < x {
      return false;
    }

    // Ray-casting check
    // Draw a ray from (x, y) to (inf, y) and count how many times it intersects with the edges of the trapezoid:
    //
    // (xtl, yt) ------------------- (xtr, yt)
    //           \                 /
    //            \               /
    //             \             /
    //     (xbl, yb) ___________ (xbr, yb)
    //
    // We can skip checking the edges (xtl, yt) -> (xtr, yt) and (xbl, yb) -> (xbr, yb)
    // since they can never intersect a horizontal ray.
    // To check for intersection, get the slope dx/dy e.g. for (xbr, yb) -> (xtr, yt):
    // dx/dy = (xtr - xbr) / (yt - yb)
    // then find the x-coord on this line at height y:
    // xbr + (y - yb) * (xtr - xbr) / (yt - yb)
    // if the value of x is less than or equal to this, then the ray intersects.
    // Since we know (yt - yb) > 0, we can rearrange the equation to avoid an expensive division operation:
    // The edge (xbr, yb) -> (xtr, yt) intersects the ray if: (x - xbr) * (yt - yb) <= (y - yb) * (xtr - xbr)
    // The edge (xbl, yb) -> (xtl, yt) intersects the ray if: (x - xbl) * (yt - yb) <= (y - yb) * (xtl - xbl)
    // The point (x, y) is inside the trapezoid if it intersects exactly 1 of these two edges.

    let intersects_r = (x - xbr) * (yt - yb) <= (y - yb) * (xtr - xbr);
    let intersects_l = (x - xbl) * (yt - yb) <= (y - yb) * (xtl - xbl);

    intersects_r ^ intersects_l
  }

  pub fn get_instance_type(&self) -> InstanceType {
    read_ptr!(self.instance_info_ptr as *const InstanceInfo, .instance_type).unwrap_or(InstanceType::Loading)
  }

  pub fn get_map_id(&self) -> Option<models::map::Map> {
    read_ptr!(self.thread.get_game_context(), .character.current_map_id)
  }

  pub fn get_current_language_id(&self) -> Option<u32> {
    read_ptr!(self.thread.get_game_context(), .character.current_language_id)
  }

  pub fn get_current_region_id(&self) -> u32 {
    unsafe { (self.current_region_id_ptr as *const u32).read_unaligned() }
  }

  pub fn get_area_info(&self, map_id: models::map::Map) -> *const AreaInfo {
    let ptr = self.area_info_ptr as *const AreaInfo;
    if ptr.is_null() {
      return ptr;
    }

    unsafe { ptr.add(map_id as usize) }
  }

  pub fn get_is_pvp_map(&self, map_id: models::map::Map) -> bool {
    let area_info = self.get_area_info(map_id);
    if area_info.is_null() {
      return false;
    }

    let area_flags = unsafe { (&raw const (*area_info).flags).read_unaligned() };
    area_flags.contains(AreaFlags::IsExplorable) && area_flags.contains(AreaFlags::IsOutpost)
  }

  pub fn get_pathing_maps(&self) -> Option<Array<PathingMap>> {
    read_ptr!(self.thread.get_game_context(), .map.map_boundary.pathing_maps_container.pathing_maps)
  }

  pub fn travel(&self, map_id: models::map::Map) {
    self.travel_advanced(
      map_id,
      self.get_current_region_id(),
      self.get_current_language_id().unwrap_or(0),
      0,
    );
  }

  pub fn travel_advanced(&self, map_id: models::map::Map, region: u32, language: u32, district: u32) {
    let message = models::ui_message::MapTravelUiMessage {
      map_id,
      region,
      language,
      district,
    };
    self.ui.send_ui_message(self.map_travel_ui_message_id, message, 0);
  }

  pub fn register_map_loaded_hook<F>(&self, func: F) -> uuid::Uuid
  where
    F: Fn(*const MapLoadedUiMessage) + Send + Sync + 'static,
  {
    self
      .ui
      .register_send_ui_message_hook(self.map_loaded_ui_message_id, move |message| {
        func(message);
      })
  }
}

pub fn init(scanner: &Scanner, thread: Arc<crate::gw::thread::Thread>, ui: Arc<crate::gw::ui::Ui>) -> Result<Map> {
  let (
    instance_info_ptr,
    map_loaded_ui_message_id,
    find_path,
    area_info_ptr,
    map_travel_ui_message_id,
    current_region_id_ptr,
  ) = parallel![
    || find_instance_info_pointer(scanner).context("Could not find instance_info pointer."),
    || find_map_loaded_ui_message_id(scanner).context("Could not find map_loaded ui message id."),
    || find_find_path_function(scanner).context("Could not find find_path function."),
    || find_area_info_pointer(scanner).context("Could not find area_info pointer."),
    || find_map_travel_ui_message_id(scanner, ui.send_ui_message_fn)
      .context("Could not find map_travel ui message id."),
    || find_current_region_id_ptr(scanner).context("Could not find current_region_id pointer."),
  ]?;

  Ok(Map {
    thread,
    ui,

    find_path_fn: unsafe { std::mem::transmute(find_path) },
    area_info_ptr,
    instance_info_ptr,
    map_loaded_ui_message_id,
    map_travel_ui_message_id,
    current_region_id_ptr,
  })
}

fn find_instance_info_pointer(scanner: &Scanner) -> Result<usize> {
  let string = r"P:\Code\Gw\AgentView\avmap.cpp";
  let address = scanner
    .scan_for_string(string)
    .or_else(|_| scanner.scan_for_string(&string.to_lowercase()))?;
  let bytes = address.to_le_bytes();

  let pattern = pattern![
    0x68, bytes[0], bytes[1], bytes[2], bytes[3], // PUSH string
  ];

  let address = scanner.scan(&pattern)? + pattern.len();
  let address = scanner.scan_from(&pattern, address)? + pattern.len();

  let pattern = pattern![
    0x51,             // PUSH ECX
    0x50,             // PUSH EAX
    0xe8, ?, ?, ?, ?, // CALL ...
    0xa3,             // MOV [instance_info_ptr], EAX
  ];

  let address = scanner.scan_from(&pattern, address)? + pattern.len();
  let instance_info_ptr = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned instance_info_ptr: 0x{:08X}", instance_info_ptr);
  Ok(instance_info_ptr)
}

fn find_map_loaded_ui_message_id(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Base\Rtl\File.cpp"), Some(r"lowestFileId"))?;

  let address = scanner.scan_back_to_function_start(address)?;
  let address = scanner.scan_for_function_call(address)?;

  let pattern = pattern![
    0x68, ?, ?, ?, ?, // PUSH
    0xe8, ?, ?, ?, ?, // CALL send_ui_message
    0x83, 0xc4, ?,    // ADD ESP, ...
  ];
  let address = scanner.scan_back_from(&pattern, address)? + 1;
  let map_loaded_ui_message_id = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned map_loaded_ui_message_id: 0x{:08X}", map_loaded_ui_message_id);
  Ok(map_loaded_ui_message_id) // 0x10000098
}

fn find_find_path_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Engine\Agent\AgTrack.cpp"), Some(r"asyncPtr"))?;

  #[rustfmt::skip]
  let pattern = pattern![
    0xd8, 0xd1,       // FCOM
    0xdf, 0xe0,       // FNSTSW AX
    0xdd, 0xd9,       // FSTP ST1
    0xf6, 0xc4, 0x01, // TEST AH, 0x1
  ];
  let address = scanner.scan_from(&pattern, address)? + pattern.len();

  let pattern = pattern![
    0xe8, ?, ?, ?, ?, // CALL find_path_function
    0x83, 0xc4, 0x18, // ADD ESP, 0x18
  ];
  let address = scanner.scan_from(&pattern, address)?;
  let find_path_function = scanner.follow_call(address)?;

  log::debug!("Scanned find_path_function: 0x{:08X}", find_path_function);
  Ok(find_path_function)
}

fn find_area_info_pointer(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Const\ConstMission.cpp"),
    Some(r"index < arrsize(s_missionClientData)"),
  )?;

  let pattern = pattern![
    0x05, ?, ?, ?, ?, // ADD EAX, area_info_ptr
    0x5d,             // POP EBP
    0xc3,             // RET
  ];

  let address = scanner.scan_from(&pattern, address)? + 1;
  let area_info_ptr = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned area_info_ptr: 0x{:08X}", area_info_ptr);
  Ok(area_info_ptr)
}

fn find_map_travel_ui_message_id(scanner: &Scanner, send_ui_message_fn: usize) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Ui\Game\Map\GmMapWorld.cpp"),
    Some(r"MISSIONS != dialogMission.mission"),
  )?;

  let pattern = pattern![
    0xe8, ?, ?, ?, ?, // CALL ...
    0x5f,             // POP EDI
    0x5e,             // POP ESI
    0x5b,             // POP EBX
  ];

  let address = scanner.scan_from(&pattern, address)?;
  let address = scanner.follow_call(address)?;

  let address = scanner.scan_for_function_call_from(send_ui_message_fn, address)?;

  let pattern = pattern![
    0x68, ?, ?, 0x00, 0x10, // PUSH map_travel_ui_message_id
  ];

  let address = scanner.scan_back_from(&pattern, address)? + 1;
  let map_travel_ui_message_id = unsafe { (address as *const usize).read_unaligned() };

  Ok(map_travel_ui_message_id)
}

fn find_current_region_id_ptr(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Gw\Ui\UiApi.cpp"), Some(r"preferredLanguage"))?;

  let pattern = pattern![
    0x83, 0xf8, 0x07, // CMP EAX, 0x7
    0x7c, ?,          // JL ...
    0xe8,             // CALL get_current_region_fn
  ];

  let address = scanner.scan_from(&pattern, address)? + pattern.len() - 1;
  let address = scanner.follow_call(address)? + 1;
  let current_region_id_ptr = unsafe { (address as *const usize).read_unaligned() };

  Ok(current_region_id_ptr)
}
