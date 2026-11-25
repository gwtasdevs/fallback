use anyhow::{Result, anyhow, bail};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

pub fn color_to_hex(color: [f32; 3]) -> String {
  let red = if color[0].is_nan() { 0. } else { color[0] };
  let green = if color[1].is_nan() { 0. } else { color[1] };
  let blue = if color[2].is_nan() { 0. } else { color[2] };

  let red = (red * 255.).round().clamp(0., 255.) as u8;
  let green = (green * 255.).round().clamp(0., 255.) as u8;
  let blue = (blue * 255.).round().clamp(0., 255.) as u8;

  format!("0x{:02x}{:02x}{:02x}", red, green, blue)
}

pub fn hex_to_color(hex: u32) -> [f32; 3] {
  let r = (hex >> 16) & 0xFF;
  let g = (hex >> 8) & 0xFF;
  let b = hex & 0xFF;

  let r = (r as f32 / 255.0).clamp(0., 1.);
  let g = (g as f32 / 255.0).clamp(0., 1.);
  let b = (b as f32 / 255.0).clamp(0., 1.);

  [r, g, b]
}

pub fn encode_unique_item_id(model_id: u32, modifiers: Vec<u32>) -> String {
  let mut bytes = vec![];

  bytes.extend(model_id.to_le_bytes());

  for modifier in modifiers {
    bytes.extend(modifier.to_le_bytes());
  }

  BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

pub fn decode_unique_item_id(encoded: &str) -> Result<(u32, Vec<u32>)> {
  let mut bytes = BASE64_URL_SAFE_NO_PAD.decode(encoded)?;
  if bytes.len() < 4 {
    bail!("Unique Item Id is invalid: {}", encoded);
  }

  let modifiers_bytes = bytes.split_off(4);
  let model_id: u32 = bytes
    .try_into()
    .map(u32::from_le_bytes)
    .map_err(|_| anyhow!("Unique Item Id is invalid: {}", encoded))?;
  let modifiers = modifiers_bytes
    .chunks(4)
    .map(|chunk| -> Result<_> { Ok(u32::from_le_bytes(chunk.try_into()?)) })
    .collect::<Result<Vec<_>>>()?;

  Ok((model_id, modifiers))
}
