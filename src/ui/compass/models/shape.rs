use crate::lua_enum;

#[derive(Clone, Copy, Debug, Default, PartialEq, strum::EnumIter, strum::FromRepr, strum::IntoStaticStr)]
#[repr(u32)]
pub enum Shape {
  Circle,
  Square,
  #[default]
  Teardrop,
  Triangle,
}

lua_enum!(Shape, u32, "Shape");

pub const SQUARE: &[[f32; 2]] = &[[1.0, 1.0], [-1.0, 1.0], [-1.0, -1.0], [1.0, -1.0], [1.0, 1.0]];

pub const TEARDROP: &[[f32; 2]] = &[
  [1.8, 0.0],
  [0.7, 0.7],
  [0.0, 1.0],
  [-0.7, 0.7],
  [-1.0, 0.0],
  [-0.7, -0.7],
  [0.0, -1.0],
  [0.7, -0.7],
  [1.8, 0.0],
];

pub const TRIANGLE: &[[f32; 2]] = &[[1.4, 0.0], [-1.0, 1.0], [-1.0, -1.0], [1.4, 0.0]];
