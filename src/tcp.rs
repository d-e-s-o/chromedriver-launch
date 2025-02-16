// Copyright (C) 2025 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Logic for parsing the `/proc/<pid>/net/tcp` file of a process.

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::net::Ipv4Addr;

use anyhow::Context as _;
use anyhow::Result;


#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TcpEntry {
  /// The local address in use.
  pub addr: Ipv4Addr,
  /// The port.
  pub port: u16,
  /// The associated TCP socket's inode.
  pub inode: u64,
}


/// Parse a line of a proc tcp file.
fn parse_tcp_line(line: &str) -> Result<TcpEntry> {
  // Lines have the following format:
  // >  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
  // >   0: 0100007F:252B 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 1000734 1 000000009dd7e836 100 0 0 10 0
  // >   1: 00000000:D431 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 5883 1 000000009861ba23 100 0 0 10 0

  let mut parts = line.split_whitespace().skip(1);
  let local_addr_str = parts
    .next()
    .context("failed to find 'local address' component")?;
  let (addr_str, port_str) = local_addr_str
    .split_once(':')
    .with_context(|| format!("encountered malformed local address in proc tcp line: {line}"))?;
  let addr = u32::from_str_radix(addr_str, 16)
    .with_context(|| format!("encountered malformed address in proc tcp line: {line}"))?
    .to_be();
  let port = u16::from_str_radix(port_str, 16)
    .with_context(|| format!("encountered malformed port number in proc tcp line: {line}"))?;

  let mut parts = parts.skip(7);
  let inode_str = parts.next().context("failed to find 'inode' component")?;
  let inode = inode_str
    .parse::<u64>()
    .with_context(|| format!("encountered malformed inode in proc tcp line: {line}"))?;

  let entry = TcpEntry {
    addr: Ipv4Addr::from(addr),
    port,
    inode,
  };
  Ok(entry)
}


#[derive(Debug)]
struct TcpEntryIter<R> {
  /// The line reader.
  reader: R,
  /// A single reused line.
  line: String,
  /// Whether or not we have read and skipped the header already.
  skipped_header: bool,
}

impl<R> Iterator for TcpEntryIter<R>
where
  R: BufRead,
{
  type Item = Result<TcpEntry>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      let () = self.line.clear();
      match self.reader.read_line(&mut self.line) {
        Err(err) => return Some(Err(err.into())),
        Ok(0) => break None,
        Ok(_) => {
          let line_str = self.line.trim();
          if !line_str.is_empty() {
            if !self.skipped_header {
              self.skipped_header = true;
            } else {
              let result = parse_tcp_line(line_str);
              break Some(result)
            }
          }
        },
      }
    }
  }
}

/// Parse a proc tcp file from the provided reader.
fn parse_file<R>(reader: R) -> impl Iterator<Item = Result<TcpEntry>>
where
  R: Read,
{
  TcpEntryIter {
    // No real rationale for the buffer capacity, other than fixing it to a
    // certain value and not making it too small to cause too many reads.
    reader: BufReader::with_capacity(16 * 1024, reader),
    line: String::new(),
    skipped_header: false,
  }
}

/// Parse the tcp file for the process with the given PID.
// TODO: Should ideally be async, but good lord...
pub(crate) fn parse(pid: u32) -> Result<impl Iterator<Item = Result<TcpEntry>>> {
  // Note that it doesn't really matter whether we use the global
  // `/proc/net/tcp` or the process specific one. The latter is
  // basically just a snapshot of the former.
  let path = format!("/proc/{pid}/net/tcp");
  let file = File::open(&path).with_context(|| format!("failed to open proc tcp file `{path}`"))?;
  let iter = parse_file(file);
  Ok(iter)
}


#[cfg(test)]
mod tests {
  use super::*;


  /// Make sure that we can parse proc tcp lines correctly.
  #[test]
  fn tcp_line_parsing() {
    let lines = r#"
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:B1AB 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 1109147 1 00000000481c5bfd 100 0 0 10 0
   1: 00000000:D431 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 5883 1 000000009861ba23 100 0 0 10 0
   2: 0C00A8C0:D29C 8B1715B2:03E1 01 00000000:00000000 02:00000F09 00000000  1000        0 852603 2 00000000f91bdecb 35 4 14 4 4
   3: 0C00A8C0:D44A 8B1715B2:03E1 01 00000000:00000000 02:00000F0A 00000000  1000        0 847558 2 00000000907a55a3 35 4 14 10 -1

"#;

    let entries = parse_file(lines.as_bytes());
    let () = entries.for_each(|entry| {
      let _entry = entry.unwrap();
    });

    let mut entries = parse_file(lines.as_bytes());
    let expected = TcpEntry {
      addr: Ipv4Addr::new(127, 0, 0, 1),
      port: 0xB1AB,
      inode: 1109147,
    };
    assert_eq!(entries.next().unwrap().unwrap(), expected);
  }
}
