use std::{
  f32::consts::FRAC_PI_2,
  sync::Arc,
  time::{Duration, Instant},
};

use anyhow::Result;
use arc_swap::ArcSwap;
use hudhook::imgui::{self, StyleColor, TreeNodeFlags};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{Debouncer, new_debouncer};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{
  gw::Gw,
  ui::{Module, common::render, config::Config},
  utils::CONFIG_PATH,
};
use models::{marker::Marker, marker_group::MarkerGroup, shape::Shape};
use watchers::files::watch_files;

pub mod models;
mod watchers;

enum CompassShift {
  None,
  Shifting { start_pos: [f32; 2] },
  Shifted { timestamp: Instant, x: f32, y: f32 },
}

pub struct Compass {
  // drop order is important here
  _debouncer: Option<Debouncer<RecommendedWatcher>>,
  markers_root: Arc<ArcSwap<MarkerGroup>>,
  shift: CompassShift,
  gw: Arc<Gw>,
  _cancel_guard: DropGuard,
}

unsafe extern "C" fn square_size_callback(data: *mut imgui::sys::ImGuiSizeCallbackData) {
  unsafe {
    (*data).DesiredSize.y = (*data).DesiredSize.x;
  }
}

impl Module for Compass {
  fn render(&mut self, ui: &imgui::Ui, config: &mut Config) -> bool {
    if !config.compass.enabled {
      return false;
    }

    let mut config_changed = false;

    let draw_list = ui.get_background_draw_list();

    if !config.lock_windows {
      let _bg = ui.push_style_color(StyleColor::WindowBg, [0.0, 0.0, 0.0, 0.0]);
      let [max_x, max_y] = ui.io().display_size;

      unsafe {
        imgui::sys::igSetNextWindowSizeConstraints(
          imgui::sys::ImVec2 { x: 100.0, y: 100.0 },
          imgui::sys::ImVec2 { x: max_x, y: max_y },
          Some(square_size_callback),
          0 as _,
        );
      }

      ui.window("##compass")
        .collapsible(false)
        .position(config.compass.position, imgui::Condition::FirstUseEver)
        .position_pivot([0.5, 0.5])
        .resizable(true)
        .scroll_bar(false)
        .size(
          [config.compass.size, config.compass.size],
          imgui::Condition::FirstUseEver,
        )
        .title_bar(false)
        .build(|| {
          let pos = ui.window_pos();
          let size = ui.window_size();
          let center_pos = [pos[0] + size[0] * 0.5, pos[1] + size[1] * 0.5];
          if center_pos != config.compass.position {
            config.compass.position = center_pos;
            config_changed = true;
          }

          if size[0] != config.compass.size {
            config.compass.size = size[0];
            config_changed = true;
          } else if size[1] != config.compass.size {
            config.compass.size = size[1];
            config_changed = true;
          }
        });
    }

    if let Some(player) = self.gw.agent.get_player() {
      let io = ui.io();
      let [mouse_x, mouse_y] = io.mouse_pos;
      let [shift_x, shift_y] = match self.shift {
        CompassShift::None => [0., 0.],
        CompassShift::Shifting { start_pos } => [mouse_x - start_pos[0], mouse_y - start_pos[1]],
        CompassShift::Shifted { x, y, .. } => [x, y],
      };
      let radius = config.compass.size * 0.5;
      let [center_x, center_y] = config.compass.position;
      let scale = radius / 5000.;
      let player_x = unsafe { (&raw const (*player).base.x).read_unaligned() };
      let player_y = unsafe { (&raw const (*player).base.y).read_unaligned() };
      let camera_rotation = FRAC_PI_2 - self.gw.camera.get_yaw();
      let rotation_cos = camera_rotation.cos();
      let rotation_sin = camera_rotation.sin();

      let transform = move |x: f32, y: f32| {
        // transforms a game coordinate into an imgui screen coordinate

        // translate so that the player position is the origin
        let x = x - player_x;
        let y = y - player_y;

        // rotate based on the camera
        let (x, y) = (x * rotation_cos - y * rotation_sin, x * rotation_sin + y * rotation_cos);

        // flip y-axis because in imgui, 0 is at the top of the screen
        let y = -y;

        // scale & translate by the user settings
        let (x, y) = (center_x + x * scale, center_y + y * scale);

        // translate by the amount shifted (by shift-click & drag)
        let x = x + shift_x;
        let y = y + shift_y;

        [x, y]
      };
      let un_transform = move |x: f32, y: f32| {
        // transforms an imgui screen coordinate into a game coordinate

        // un-translate by the amount shifted (by shift-click & drag)
        let x = x - shift_x;
        let y = y - shift_y;

        // un-translate and un-scale the user settings
        let (x, y) = ((x - center_x) / scale, (y - center_y) / scale);

        // un-flip y-axis because in imgui, 0 is at the top of the screen
        let y = -y;

        // un-rotate the camera rotation
        let (x, y) = (x * rotation_cos + y * rotation_sin, y * rotation_cos - x * rotation_sin);

        // un-translate from the player position at the origin
        let x = x + player_x;
        let y = y + player_y;

        [x, y]
      };

      let agents = self.gw.agent.agents();
      let pathing_maps = self.gw.map.get_pathing_maps();

      let min_bounds = [center_x - radius, center_y - radius];
      let max_bounds = [center_x + radius, center_y + radius];

      draw_list.with_clip_rect(min_bounds, max_bounds, || {
        // pathing maps
        if let Some(pathing_maps) = pathing_maps {
          for map in pathing_maps.iter() {
            if map.is_null() {
              continue;
            }

            let trapezoid_count = unsafe { (&raw const (*map).trapezoid_count).read_unaligned() } as usize;
            for i in 0..trapezoid_count {
              let trapezoids = unsafe { (&raw const (*map).trapezoids).read_unaligned() };
              if trapezoids.is_null() {
                continue;
              }
              let trapezoid = unsafe { trapezoids.add(i) };
              if trapezoid.is_null() {
                continue;
              }

              let xtl = unsafe { (&raw const (*trapezoid).xtl).read_unaligned() };
              let xbl = unsafe { (&raw const (*trapezoid).xbl).read_unaligned() };
              let xtr = unsafe { (&raw const (*trapezoid).xtr).read_unaligned() };
              let xbr = unsafe { (&raw const (*trapezoid).xbr).read_unaligned() };
              let yt = unsafe { (&raw const (*trapezoid).yt).read_unaligned() };
              let yb = unsafe { (&raw const (*trapezoid).yb).read_unaligned() };
              draw_list
                .add_polyline(
                  vec![
                    transform(xbl, yb),
                    transform(xbr, yb),
                    transform(xtr, yt),
                    transform(xtl, yt),
                  ],
                  [1., 1., 1., 1.], // TODO: from config
                )
                .filled(true)
                .build();
            }
          }
        }

        let markers_root = self.markers_root.load();

        // static markers
        let mut markers = markers_root.resolve_static_markers();
        markers.sort_by_key(|&marker| match marker {
          Marker::Circle { draw_priority, .. } => draw_priority.unwrap_or(0),
          Marker::Line { draw_priority, .. } => draw_priority.unwrap_or(0),
          _ => 0,
        });

        for marker in markers {
          match marker {
            Marker::Circle {
              x,
              y,
              radius,
              border_color,
              border_opacity,
              border_thickness,
              fill_color,
              fill_opacity,
              ..
            } => {
              let position = transform(*x, *y);
              let border_color = border_color.unwrap_or([0., 0., 0.]);
              let border_opacity = border_opacity.unwrap_or(1.);
              let border_thickness = border_thickness.unwrap_or(1.);
              let fill_color = fill_color.unwrap_or([0., 0., 0.]);
              let fill_opacity = fill_opacity.unwrap_or(1.);

              if fill_opacity > 0. {
                draw_list
                  .add_circle(
                    position,
                    radius * scale,
                    [fill_color[0], fill_color[1], fill_color[2], fill_opacity],
                  )
                  .filled(true)
                  .build();
              }

              if border_opacity > 0. {
                draw_list
                  .add_circle(
                    position,
                    radius * scale,
                    [border_color[0], border_color[1], border_color[2], border_opacity],
                  )
                  .thickness(border_thickness)
                  .build();
              }
            }
            Marker::Line {
              points,
              border_color,
              border_opacity,
              border_thickness,
              fill_color,
              fill_opacity,
              ..
            } => {
              let points = points.iter().map(|(x, y)| transform(*x, *y)).collect::<Vec<_>>();
              let border_color = border_color.unwrap_or([0., 0., 0.]);
              let border_opacity = border_opacity.unwrap_or(1.);
              let border_thickness = border_thickness.unwrap_or(1.);
              let fill_color = fill_color.unwrap_or([0., 0., 0.]);
              let fill_opacity = fill_opacity.unwrap_or(1.);

              if fill_opacity > 0. {
                draw_list
                  .add_polyline(
                    points.clone(),
                    [fill_color[0], fill_color[1], fill_color[2], fill_opacity],
                  )
                  .filled(true)
                  .build();
              }

              if border_opacity > 0. {
                draw_list
                  .add_polyline(
                    points,
                    [border_color[0], border_color[1], border_color[2], border_opacity],
                  )
                  .thickness(border_thickness)
                  .build();
              }
            }
            _ => {}
          }
        }

        // agent markers
        let mut agent_markers = vec![];
        for agent in agents.into_iter() {
          if agent.is_null() {
            continue;
          }
          let agent_id = unsafe { (&raw const (*agent).agent_id).read_unaligned() };
          let agent_x = unsafe { (&raw const (*agent).x).read_unaligned() };
          let agent_y = unsafe { (&raw const (*agent).y).read_unaligned() };
          let rotation_cos = unsafe { (&raw const (*agent).rotation_cos).read_unaligned() };
          let rotation_sin = unsafe { (&raw const (*agent).rotation_sin).read_unaligned() };

          let markers = markers_root
            .resolve_agent_markers(agent_id)
            .into_iter()
            .map(|marker| (marker, agent_x, agent_y, rotation_cos, rotation_sin));
          agent_markers.extend(markers);
        }

        agent_markers.sort_by_key(|(marker, ..)| marker.draw_priority.unwrap_or(0));

        for (marker, agent_x, agent_y, rotation_cos, rotation_sin) in agent_markers {
          let shape = marker.shape.unwrap_or(Shape::default());
          let size = marker.size.unwrap_or(50.);
          let border_color = marker.border_color.unwrap_or([0., 0., 0.]);
          let border_opacity = marker.border_opacity.unwrap_or(1.);
          let border_thickness = marker.border_thickness.unwrap_or(1.);
          let fill_color = marker.fill_color.unwrap_or([0., 0., 0.]);
          let fill_opacity = marker.fill_opacity.unwrap_or(1.);

          let draw_polyline_shape = |points: &[[f32; 2]]| {
            let points = points
              .iter()
              .map(|[x, y]| [x * rotation_cos - y * rotation_sin, x * rotation_sin + y * rotation_cos])
              .map(|[x, y]| transform(agent_x + size * x, agent_y + size * y))
              .collect::<Vec<_>>();

            draw_list
              .add_polyline(
                points.clone(),
                [fill_color[0], fill_color[1], fill_color[2], fill_opacity],
              )
              .filled(true)
              .build();
            draw_list
              .add_polyline(
                points,
                [border_color[0], border_color[1], border_color[2], border_opacity],
              )
              .thickness(border_thickness)
              .build();
          };

          match shape {
            Shape::Circle => {
              let position = transform(agent_x, agent_y);
              let size = size * scale;

              draw_list
                .add_circle(
                  position,
                  size,
                  [fill_color[0], fill_color[1], fill_color[2], fill_opacity],
                )
                .filled(true)
                .build();
              draw_list
                .add_circle(
                  position,
                  size,
                  [border_color[0], border_color[1], border_color[2], border_opacity],
                )
                .thickness(border_thickness)
                .build();
            }
            Shape::Square => draw_polyline_shape(models::shape::SQUARE),
            Shape::Teardrop => draw_polyline_shape(models::shape::TEARDROP),
            Shape::Triangle => draw_polyline_shape(models::shape::TRIANGLE),
          }
        }

        // TODO: shadow-step lines
      });

      // if io.want_capture_mouse {
      //   return config_changed;
      // }

      let is_mouse_over_compass =
        mouse_x >= min_bounds[0] && mouse_x <= max_bounds[0] && mouse_y >= min_bounds[1] && mouse_y <= max_bounds[1];

      if is_mouse_over_compass && ui.is_mouse_clicked(imgui::MouseButton::Left) {
        if ui.is_key_down(imgui::Key::LeftCtrl) || ui.is_key_down(imgui::Key::RightCtrl) {
          let [x, y] = un_transform(mouse_x, mouse_y);

          let mut agent_id = 0;
          let mut min_sqr_distance = 300. * 300.;
          for agent in self.gw.agent.agents().into_iter() {
            if agent.is_null() {
              continue;
            }

            let agent_x = unsafe { (&raw const (*agent).x).read_unaligned() };
            let agent_y = unsafe { (&raw const (*agent).y).read_unaligned() };

            let diff_x = agent_x - x;
            let diff_y = agent_y - y;

            let sqr_distance = diff_x * diff_x + diff_y * diff_y;
            if sqr_distance < min_sqr_distance {
              min_sqr_distance = sqr_distance;
              agent_id = unsafe { (&raw const (*agent).agent_id).read_unaligned() };
            }
          }

          if agent_id != 0 {
            self.gw.agent.change_target(agent_id as usize);
          }
        } else if ui.is_key_down(imgui::Key::LeftAlt) || ui.is_key_down(imgui::Key::RightAlt) {
          let [x, y] = un_transform(mouse_x, mouse_y);
          self.gw.player.move_to(x, y);
        }
      }

      let is_mouse_down = ui.is_mouse_down(imgui::MouseButton::Left);
      let is_shift_down = ui.is_key_down(imgui::Key::LeftShift) || ui.is_key_down(imgui::Key::RightShift);
      match self.shift {
        CompassShift::None => {
          if is_mouse_over_compass && is_mouse_down && is_shift_down {
            self.shift = CompassShift::Shifting {
              start_pos: [mouse_x, mouse_y],
            };
          }
        }
        CompassShift::Shifting { start_pos } => {
          if !is_mouse_down || !is_shift_down {
            self.shift = CompassShift::Shifted {
              timestamp: Instant::now(),
              x: mouse_x - start_pos[0],
              y: mouse_y - start_pos[1],
            };
          }
        }
        CompassShift::Shifted { timestamp, x, y } => {
          if is_mouse_over_compass && is_mouse_down && is_shift_down {
            self.shift = CompassShift::Shifting {
              start_pos: [mouse_x - x, mouse_y - y],
            };
          } else if timestamp.elapsed() > Duration::from_secs(10) {
            self.shift = CompassShift::None;
          }
        }
      }
    }

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

    if ui.checkbox("##enabled", &mut config.compass.enabled) {
      config_changed = true;
    }
    if ui.is_item_hovered() {
      ui.tooltip_text(if config.compass.enabled {
        "Disable compass"
      } else {
        "Enable compass"
      });
    }

    ui.set_cursor_pos(start_pos);

    let tree = {
      let _token = ui.push_style_color(imgui::StyleColor::FrameBg, [1., 0., 0., 1.]);
      ui.tree_node_config("Compass")
        .flags(TreeNodeFlags::FRAMED | TreeNodeFlags::OPEN_ON_ARROW | TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
        .push()
    };

    if let Some(_token) = tree {
      ui.indent();

      ui.text("No settings available.");

      // if ui.slider("Scale", 0., 1000., &mut config.compass.size) {
      //   config_changed = true;
      // }

      // if ui.slider("X", 0., 5000., &mut config.compass.position[0]) {
      //   config_changed = true;
      // }
      // if ui.slider("Y", 0., 3000., &mut config.compass.position[1]) {
      //   config_changed = true;
      // }

      ui.unindent();
    }

    config_changed
  }
}

pub fn init(gw: Arc<Gw>, lua: Arc<mlua::Lua>, cancel_token: CancellationToken) -> Result<Compass> {
  let markers_path = CONFIG_PATH.join("markers");
  std::fs::create_dir_all(&markers_path)?;

  let markers_root = Arc::new(ArcSwap::from_pointee(MarkerGroup::new(
    lua.clone(),
    markers_path.join("_.lua"),
  )));
  let markers_root_ref = Arc::clone(&markers_root);

  let (tx, on_file_event) = tokio::sync::mpsc::unbounded_channel();
  let mut debouncer = new_debouncer(Duration::from_millis(500), move |result| match result {
    Ok(events) => {
      let _ = tx.send(events);
    }
    Err(err) => {
      log::warn!("File watcher sent an error: {:?}.", err);
    }
  })?;
  debouncer.watcher().watch(&markers_path, RecursiveMode::Recursive)?;
  log::debug!("File watcher initialized.");

  let _cancel_guard = cancel_token.clone().drop_guard();
  tokio::spawn(async move {
    let mut set = tokio::task::JoinSet::new();

    set.spawn(watch_files(
      Arc::clone(&markers_root_ref),
      Arc::clone(&lua),
      on_file_event,
    ));

    cancel_token.cancelled().await;
    set.shutdown().await;
  });

  Ok(Compass {
    _debouncer: Some(debouncer),
    markers_root,
    shift: CompassShift::None,
    gw,
    _cancel_guard,
  })
}
