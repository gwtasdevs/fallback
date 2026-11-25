use std::sync::Arc;

use anyhow::{Context, Result};

use crate::gw::scan::Scanner;
use crate::{parallel, pattern};

pub struct Packet {
  thread: Arc<crate::gw::thread::Thread>,

  game_server_object_ptr: usize,
  send_packet_fn: unsafe extern "cdecl" fn(game_server_object: usize, size: usize, packet: usize) -> (),
}

impl Packet {
  pub fn send_packet<T: Send + 'static>(&self, packet: T) {
    let func = self.send_packet_fn;
    let game_server_object_ptr = self.game_server_object_ptr;

    self.thread.fire_and_forget(move || {
      let game_server_object = unsafe { (game_server_object_ptr as *const usize).read_unaligned() };
      unsafe { func(game_server_object, size_of::<T>(), &raw const packet as usize) };
    });
  }

  pub fn send_sized_packet(&self, packet: Vec<usize>) {
    let func = self.send_packet_fn;
    let game_server_object_ptr = self.game_server_object_ptr;

    self.thread.fire_and_forget(move || unsafe {
      let game_server_object = (game_server_object_ptr as *const usize).read_unaligned();
      func(
        game_server_object,
        packet.len() * size_of::<usize>(),
        packet.as_ptr() as usize,
      );
    });
  }
}

pub fn init(scanner: &Scanner, thread: Arc<crate::gw::thread::Thread>) -> Result<Packet> {
  let (game_server_object, send_packet) = parallel![
    || find_game_server_object(scanner).context("Could not find game_server object."),
    || find_send_packet_function(scanner).context("Could not find send_packet function."),
  ]?;

  Ok(Packet {
    thread,

    game_server_object_ptr: game_server_object,
    send_packet_fn: unsafe { std::mem::transmute(send_packet) },
  })
}

fn find_game_server_object(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(
    Some(r"P:\Code\Gw\Net\Cli\GcGameCmd.cpp"),
    Some(r"No valid case for switch variable 'code'"),
  )?;

  let pattern = pattern![
    0xa1, ?, ?, ?, ?, // MOV EAX, game_server_object
    0x85, 0xc0,       // TEST EAX, EAX
    0x74,             // JZ ...
  ];
  let address = scanner.scan_back_from(&pattern, address)? + 1;
  let game_server_object = unsafe { (address as *const usize).read_unaligned() };

  log::debug!("Scanned game_server_object: 0x{:08X}", game_server_object);
  Ok(game_server_object)
}

fn find_send_packet_function(scanner: &Scanner) -> Result<usize> {
  let address = scanner.scan_for_assertion(Some(r"P:\Code\Net\Msg\MsgConn.cpp"), Some(r"bytes >= sizeof(dword)"))?;
  let send_packet_function = scanner.scan_back_to_function_start(address)?;

  log::debug!("Scanned send_packet_function: 0x{:08X}", send_packet_function);
  Ok(send_packet_function)
}
