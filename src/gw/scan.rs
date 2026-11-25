use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use windows::Win32::{
  Foundation::HMODULE,
  System::Diagnostics::Debug::{IMAGE_NT_HEADERS32, IMAGE_SECTION_HEADER, ImageNtHeader},
};

#[macro_export]
macro_rules! pattern {
  (@acc [$($acc:tt)*] ?, $($rest:tt)+) => {
    pattern!(@acc [$($acc)* None,] $($rest)*)
  };

  (@acc [$($acc:tt)*] ? $(,)?) => {
    [$($acc)* None,]
  };

  (@acc [$($acc:tt)*] $x:expr, $($rest:tt)+) => {
    pattern!(@acc [$($acc)* Some($x as u8),] $($rest)*)
  };

  (@acc [$($acc:tt)*] $x:expr $(,)?) => {
    [$($acc)* Some($x as u8),]
  };

  ($($rest:tt)*) => {
    pattern!(@acc [] $($rest)*)
  };
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum Section {
  Data,
  RData,
  Text,
}

pub struct Scanner {
  // 'static lifetime because these point to the executable memory,
  // which live for the duration of the process.
  sections: HashMap<Section, (usize, usize, &'static [u8])>,
  assert_function: usize,
}

impl Scanner {
  pub unsafe fn new(hmodule: HMODULE) -> Result<Self> {
    let nt_headers = unsafe { ImageNtHeader(hmodule.0) };
    if nt_headers.is_null() {
      bail!("ImageNtHeader is null.");
    }

    let sections = unsafe {
      let first_header = nt_headers
        .byte_add(std::mem::offset_of!(IMAGE_NT_HEADERS32, OptionalHeader))
        .byte_add(nt_headers.read_unaligned().FileHeader.SizeOfOptionalHeader as usize);

      std::slice::from_raw_parts(
        first_header as *const IMAGE_SECTION_HEADER,
        nt_headers.read_unaligned().FileHeader.NumberOfSections as usize,
      )
    };

    let sections = sections
      .iter()
      .filter_map(|section| {
        let name = if section.Name.starts_with(b".data") {
          Some(Section::Data)
        } else if section.Name.starts_with(b".rdata") {
          Some(Section::RData)
        } else if section.Name.starts_with(b".text") {
          Some(Section::Text)
        } else {
          None
        };

        if let Some(name) = name {
          let start = unsafe { hmodule.0.byte_add(section.VirtualAddress as usize) } as *const u8;
          let size = unsafe { section.Misc.VirtualSize as usize };
          let end = start as usize + size;
          let data = unsafe { std::slice::from_raw_parts(start, size) };
          return Some((name, (start as usize, end, data)));
        }

        None
      })
      .collect::<HashMap<_, _>>();

    let (text_start, _text_end, text_data) = sections.get(&Section::Text).ok_or(anyhow!("No .text section found."))?;
    let (rdata_start, _rdata_end, rdata_data) = sections
      .get(&Section::RData)
      .ok_or(anyhow!("No .rdata section found."))?;

    let file = Scanner::scan_data(
      *rdata_start,
      rdata_data,
      &to_pattern(r"P:\Code\Engine\Frame\FrApi.cpp"),
      false,
    )
    .or_else(|_| {
      Scanner::scan_data(
        *rdata_start,
        rdata_data,
        &to_pattern(&r"P:\Code\Engine\Frame\FrApi.cpp".to_lowercase()),
        false,
      )
    })?;
    let msg = Scanner::scan_data(*rdata_start, rdata_data, &to_pattern(r"!s_bufferBits"), false)?;

    let file = file.to_le_bytes();
    let msg = msg.to_le_bytes();

    #[rustfmt::skip]
    let pattern = pattern![
      0xba, file[0], file[1], file[2], file[3], // MOV EDX, file
      0xb9, msg[0], msg[1], msg[2], msg[3],     // MOV ECX, message
      0xe8,                                     // CALL assert_function
    ];
    let address = Scanner::scan_data(*text_start, text_data, &pattern, false)? + pattern.len();
    let relative_address = unsafe { (address as *const i32).read_unaligned() };
    let assert_function: usize = (address as i64 + 4 + relative_address as i64).try_into()?;
    log::debug!("Scanned assert_function: 0x{:08X}", assert_function);

    Ok(Self {
      sections,
      assert_function,
    })
  }

  /// Scans for an address that matches the pattern of bytes.
  pub fn scan(&self, pattern: &[Option<u8>]) -> Result<usize> {
    self.scan_section(&Section::Text, pattern)
  }

  /// Scans for all addresses that match the pattern of bytes.
  pub fn scan_multi(&self, pattern: &[Option<u8>]) -> Result<Vec<usize>> {
    if let Some((start, _end, data)) = self.sections.get(&Section::Text) {
      return Scanner::scan_data_multi(*start, data, pattern);
    }

    bail!("No {:?} section specified.", Section::Text);
  }

  pub fn scan_section(&self, section: &Section, pattern: &[Option<u8>]) -> Result<usize> {
    if let Some((start, _end, data)) = self.sections.get(section) {
      return Scanner::scan_data(*start, data, pattern, false);
    }

    bail!("No {:?} section specified.", section);
  }

  /// Scans to find the first location where the function at `address` is called from.
  pub fn scan_for_function_call(&self, address: usize) -> Result<usize> {
    if let Some((start, _end, data)) = self.sections.get(&Section::Text) {
      return self.scan_data_for_function_call(*start, data, address, false);
    }

    bail!("No .text section found.");
  }

  /// Scans to find all locations where the function at `address` is called from.
  #[expect(unused)]
  pub fn scan_for_function_calls(&self, address: usize) -> Result<Vec<usize>> {
    if let Some((start, _end, data)) = self.sections.get(&Section::Text) {
      return self.scan_data_for_function_calls(*start, data, address);
    }

    bail!("No .text section found.");
  }

  /// Scans for a pattern of bytes up to 0x300 bytes after `start`.
  pub fn scan_from(&self, pattern: &[Option<u8>], start: usize) -> Result<usize> {
    self.scan_from_until(pattern, start, start + 0x300)
  }

  /// Scans for a pattern of bytes up to 0x300 bytes before `start`.
  pub fn scan_back_from(&self, pattern: &[Option<u8>], start: usize) -> Result<usize> {
    self.scan_from_until(pattern, start, start - 0x300)
  }

  /// Scans for a call to the function at `function_address` up to 0x300 bytes after `start`.
  pub fn scan_for_function_call_from(&self, function_address: usize, start: usize) -> Result<usize> {
    self.scan_for_function_call_from_until(function_address, start, start + 0x300)
  }

  /// Scans for a call to the function at `function_address` up to 0x300 bytes before `start`.
  pub fn scan_for_function_call_back_from(&self, function_address: usize, start: usize) -> Result<usize> {
    self.scan_for_function_call_from_until(function_address, start, start - 0x300)
  }

  /// Scans for a string within the .rdata section.
  pub fn scan_for_string(&self, string: &str) -> Result<usize> {
    let mut pattern = string.bytes().map(Some).collect::<Vec<_>>();
    pattern.push(Some(0)); // end with null terminator

    self.scan_section(&Section::RData, &pattern)
  }

  /// Scans for a utf16 encoded string within the .rdata section.
  pub fn scan_for_wide_string(&self, string: &str) -> Result<usize> {
    let mut pattern = string
      .encode_utf16()
      .flat_map(|c| c.to_le_bytes())
      .map(Some)
      .collect::<Vec<_>>();

    // end with null terminator
    pattern.push(Some(0));
    pattern.push(Some(0));

    self.scan_section(&super::scan::Section::RData, &pattern)
  }

  /// Scans forward from `start` up to 0x300 bytes until reaching the start of a function.
  #[expect(unused)]
  pub fn scan_to_next_function_start(&self, start: usize) -> Result<usize> {
    #[rustfmt::skip]
    let pattern = pattern![
      0x55,       // PUSH EBP
      0x8b, 0xec, // MOV EBP, ESP
    ];
    self.scan_from_until(&pattern, start, start + 0x300)
  }

  /// Scans backward from `start` up to 0x300 bytes until reaching the start of a function.
  pub fn scan_back_to_function_start(&self, start: usize) -> Result<usize> {
    #[rustfmt::skip]
    let pattern = pattern![
      0x55,       // PUSH EBP
      0x8b, 0xec, // MOV EBP, ESP
    ];
    self.scan_from_until(&pattern, start, start - 0x300)
  }

  /// Scans backward from `start` up to `until` bytes until reaching the start of a function.
  pub fn scan_back_to_function_start_until(&self, start: usize, until: usize) -> Result<usize> {
    #[rustfmt::skip]
    let pattern = pattern![
      0x55,       // PUSH EBP
      0x8b, 0xec, // MOV EBP, ESP
    ];
    self.scan_from_until(&pattern, start, start - until)
  }

  /// Scans forward from `start` up to 0x300 bytes until reaching the end of a function.
  pub fn scan_to_function_end(&self, start: usize) -> Result<usize> {
    #[rustfmt::skip]
    let pattern = pattern![
      0x8b, 0xe5, // MOV ESP, EBP
      0x5d,       // POP EBP
      0xc3,       // RET
    ];

    if let Ok(address) = self.scan_from_until(&pattern, start, start + 0x300) {
      return Ok(address + 3);
    }

    #[rustfmt::skip]
    let pattern = pattern![
      0xc3,      // RET
      0xcc, 0xcc // ...
    ];

    self.scan_from_until(&pattern, start, start + 0x300)
  }

  /// Scans backward from `start` up to 0x300 bytes until reaching the end of a function.
  #[expect(unused)]
  pub fn scan_back_to_function_end(&self, start: usize) -> Result<usize> {
    #[rustfmt::skip]
    let pattern = pattern![
      0x8b, 0xe5, // MOV ESP, EBP
      0x5d,       // POP EBP
      0xc3,       // RET
    ];

    if let Ok(address) = self.scan_from_until(&pattern, start, start - 0x300) {
      return Ok(address + 3);
    }

    #[rustfmt::skip]
    let pattern = pattern![
      0xc3,      // RET
      0xcc, 0xcc // ...
    ];

    self.scan_from_until(&pattern, start, start - 0x300)
  }

  fn scan_from_until(&self, pattern: &[Option<u8>], from: usize, until: usize) -> Result<usize> {
    if from <= until {
      let data = unsafe { std::slice::from_raw_parts(from as *const u8, until - from) };
      Scanner::scan_data(from, data, pattern, false)
    } else {
      let data = unsafe { std::slice::from_raw_parts(until as *const u8, from - until) };
      Scanner::scan_data(until, data, pattern, true)
    }
  }

  fn scan_for_function_call_from_until(&self, function_address: usize, from: usize, until: usize) -> Result<usize> {
    if from <= until {
      let data = unsafe { std::slice::from_raw_parts(from as *const u8, until - from) };
      self.scan_data_for_function_call(from, data, function_address, false)
    } else {
      let data = unsafe { std::slice::from_raw_parts(until as *const u8, from - until) };
      self.scan_data_for_function_call(until, data, function_address, true)
    }
  }

  fn scan_data(start: usize, data: &[u8], pattern: &[Option<u8>], reverse: bool) -> Result<usize> {
    let predicate = |window: &[u8]| {
      window
        .iter()
        .zip(pattern)
        .all(|(data_byte, pattern_byte)| match pattern_byte {
          Some(pattern_byte) => data_byte == pattern_byte,
          None => true,
        })
    };
    let position = if reverse {
      data.windows(pattern.len()).rposition(predicate)
    } else {
      data.windows(pattern.len()).position(predicate)
    };

    if let Some(position) = position {
      return Ok(start + position);
    }

    bail!("No match for pattern {}.", display_pattern(pattern));
  }

  fn scan_data_multi(start: usize, data: &[u8], pattern: &[Option<u8>]) -> Result<Vec<usize>> {
    let results = data
      .windows(pattern.len())
      .enumerate()
      .filter(|(_, window)| {
        window
          .iter()
          .zip(pattern)
          .all(|(data_byte, pattern_byte)| match pattern_byte {
            Some(pattern_byte) => data_byte == pattern_byte,
            None => true,
          })
      })
      .map(|(position, _)| start + position)
      .collect::<Vec<_>>();

    Ok(results)
  }

  fn scan_data_for_function_call(
    &self,
    start: usize,
    data: &[u8],
    function_address: usize,
    reverse: bool,
  ) -> Result<usize> {
    let process = |i: usize| -> Result<Option<usize>> {
      match data[i] {
        0xe8   // CALL rel32
        | 0xe9 // JMP rel32
        => {
          let relative_address = (function_address as i64 - (start + i + 5) as i64) as i32;
          if i32::from_le_bytes(data[i+1..i+5].try_into()?) == relative_address {
            return Ok(Some(start + i));
          }

          Ok(None)
        },
        _ => Ok(None),
      }
    };

    if reverse {
      for i in (0..data.len() - 5).rev() {
        if let Some(result) = process(i)? {
          return Ok(result);
        }
      }
    } else {
      for i in 0..data.len() - 5 {
        if let Some(result) = process(i)? {
          return Ok(result);
        }
      }
    }

    bail!("No calls found to function at address {}.", function_address);
  }

  fn scan_data_for_function_calls(&self, start: usize, data: &[u8], function_address: usize) -> Result<Vec<usize>> {
    let mut matches = vec![];

    for i in 0..data.len() - 5 {
      match data[i] {
        0xe8   // CALL rel32
        | 0xe9 // JMP rel32
        => {
          let relative_address = (function_address as i64 - (start + i + 5) as i64) as i32;
          if i32::from_le_bytes(data[i+1..i+5].try_into()?) == relative_address {
            matches.push(start + i);
          }
        },
        _ => (),
      }
    }

    Ok(matches)
  }

  pub fn scan_for_assertion(&self, file: Option<&str>, message: Option<&str>) -> Result<usize> {
    let (file_pattern, message_pattern) = self.get_assertion_patterns(file, message)?;

    self
      .scan(&[file_pattern, message_pattern].concat())
      .or_else(|_| self.scan(&[message_pattern, file_pattern].concat()))
  }

  pub fn scan_for_assertions(&self, file: Option<&str>, message: Option<&str>) -> Result<Vec<usize>> {
    let (file_pattern, message_pattern) = self.get_assertion_patterns(file, message)?;

    let mut results = self.scan_multi(&[file_pattern, message_pattern].concat())?;
    results.extend(self.scan_multi(&[message_pattern, file_pattern].concat())?);

    Ok(results)
  }

  pub fn scan_for_assertion_call(&self, file: &str, message: &str) -> Result<usize> {
    if let Some(text) = self.sections.get(&Section::Text)
      && let Some(rdata) = self.sections.get(&Section::RData)
    {
      let (start, _end, data) = text;
      let (rdata_start, rdata_end, _data) = rdata;

      fn read_string(address: usize) -> String {
        let cstr = unsafe { std::ffi::CStr::from_ptr(address as *const i8) };
        cstr.to_string_lossy().into()
      }

      let pattern = pattern![
        0xba, ?, ?, ?, ?, // MOV EDX, file
        0xb9, ?, ?, ?, ?, // MOV ECX, message
        0xe8, ?, ?, ?, ?, // CALL assert_function
      ];

      for i in 0..(data.len() - pattern.len()) {
        let is_match =
          data[i..i + pattern.len()]
            .iter()
            .zip(pattern)
            .all(|(data_byte, pattern_byte)| match pattern_byte {
              Some(pattern_byte) => *data_byte == pattern_byte,
              None => true,
            });

        if !is_match {
          continue;
        }

        let relative_address = (self.assert_function as i64 - (*start + i + pattern.len()) as i64) as i32;
        if i32::from_le_bytes(data[i + 11..i + 15].try_into()?) != relative_address {
          continue;
        }

        let file_address = usize::from_le_bytes(data[i + 1..i + 5].try_into()?);
        if file_address < *rdata_start || file_address > *rdata_end {
          continue;
        }
        let file_string = read_string(file_address);

        let message_address = usize::from_le_bytes(data[i + 6..i + 10].try_into()?);
        if message_address < *rdata_start || message_address > *rdata_end {
          continue;
        }
        let message_string = read_string(message_address);

        if (file_string == file || file_string == file.to_lowercase()) && message_string == message {
          return Ok(*start + i);
        }
      }
    }

    bail!("No .text or .rdata section found.");
  }

  #[expect(clippy::type_complexity)]
  fn get_assertion_patterns(
    &self,
    file: Option<&str>,
    message: Option<&str>,
  ) -> Result<([Option<u8>; 5], [Option<u8>; 5])> {
    // MOV EDX, file
    let file_pattern = if let Some(file) = file {
      let address = self
        .scan_for_string(file)
        .or_else(|_| self.scan_for_string(&file.to_lowercase()))?;
      let bytes = address.to_le_bytes();

      pattern!(0xba, bytes[0], bytes[1], bytes[2], bytes[3])
    } else {
      pattern![0xba, ?, ?, ?, ?]
    };

    // MOV ECX, message
    let message_pattern = if let Some(message) = message {
      let address = self.scan_for_string(message)?;
      let bytes = address.to_le_bytes();

      pattern!(0xb9, bytes[0], bytes[1], bytes[2], bytes[3])
    } else {
      pattern![0xb9, ?, ?, ?, ?]
    };

    Ok((file_pattern, message_pattern))
  }

  pub fn follow_call(&self, call_address: usize) -> Result<usize> {
    if call_address == 0 {
      bail!("Invalid call_address: {}.", call_address);
    }

    let data = unsafe { std::slice::from_raw_parts(call_address as *const u8, 6) };

    match data[0] {
      0xe8   // CALL rel32
      | 0xe9 // JMP rel32
      => {
        let relative_address = i32::from_le_bytes(data[1..5].try_into()?);
        let address: usize = (call_address as i64 + 5 + relative_address as i64).try_into()?;

        // check for nested JMP instructions.
        let first_byte = unsafe { (address as *const u8).read_unaligned() };
        if first_byte == 0xE9 {
          return self.follow_call(address);
        }

        Ok(address)
      },
      0x72   // JB rel8
      | 0x73 // JNB rel8
      | 0x74 // JZ rel8
      | 0x75 // JNZ rel8
      | 0x76 // JNA rel8
      | 0x77 // JA rel8
      | 0x78 // JS rel8
      | 0x79 // JNS rel8
      | 0x7a // JP rel8
      | 0x7b // JNP rel8
      | 0x7c // JL rel8
      | 0x7d // JNL rel8
      | 0x7e // JNG rel8
      | 0x7f // JG rel8
      | 0xe3 // JCXZ rel8
      | 0xeb // JMP rel8
      => {
        let relative_address = data[1].cast_signed();
        let address: usize = (call_address as i64 + 2 + relative_address as i64).try_into()?;
        Ok(address)
      },
      0x0f => {
        match data[1] {
          #[expect(clippy::manual_range_patterns)]
          0x80   // JO rel32
          | 0x81 // JNO rel32
          | 0x82 // JB rel32
          | 0x83 // JNB rel32
          | 0x84 // JZ rel32
          | 0x85 // JNZ rel32
          | 0x86 // JNA rel32
          | 0x87 // JA rel32
          | 0x88 // JS rel32
          | 0x89 // JNS rel32
          | 0x8a // JP rel32
          | 0x8b // JNP rel32
          | 0x8c // JL rel32
          | 0x8d // JNL rel32
          | 0x8e // JNG rel32
          | 0x8f // JG rel32
          => {
            let relative_address = i32::from_le_bytes(data[2..6].try_into()?);
            let address: usize = (call_address as i64 + 6 + relative_address as i64).try_into()?;
            Ok(address)
          },
          _ => bail!("Not a valid near call instruction {:X?}", data),
        }
      },
      _ => bail!("Not a valid near call instruction {:X?}", data),
    }
  }

  // pub fn follow_calls(&self, start: usize, patterns: &[(impl AsRef<[Option<u8>]>, i32)]) -> Result<usize> {
  //   let mut address = start;
  //   for (pattern, offset) in patterns {
  //     let pattern_address = self.scan_from(pattern.as_ref(), address)?;

  //     let call_address: usize = (pattern_address as i64 + *offset as i64).try_into()?;
  //     address = self.follow_call(call_address)?;
  //   }

  //   Ok(address)
  // }
}

fn display_pattern(pattern: &[Option<u8>]) -> String {
  let chars = pattern
    .iter()
    .map(|byte| match byte {
      Some(byte) => format!("0x{:02X}", byte),
      None => "?".to_string(),
    })
    .collect::<Vec<_>>();

  format!("[{}]", chars.join(", "))
}

fn to_pattern(string: &str) -> Vec<Option<u8>> {
  let mut pattern = string.bytes().map(Some).collect::<Vec<_>>();
  pattern.push(Some(0)); // end with null terminator
  pattern
}
