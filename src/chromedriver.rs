// Copyright (C) 2025 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use anyhow::bail;
use anyhow::Context as _;
use anyhow::Result;

use crate::socket;
use crate::tcp;


/// The name of the `chromedriver` binary.
const CHROME_DRIVER: &str = "chromedriver";
/// The timeout used when searching for a bound local port.
const PORT_FIND_TIMEOUT: Duration = Duration::from_secs(30);


fn find_localhost_port(pid: u32) -> Result<u16> {
  let start = Instant::now();

  // Wait for the driver process to bind to a local host address.
  let port = loop {
    let inodes = socket::socket_inodes(pid)?.collect::<Result<HashSet<_>>>()?;
    let result = tcp::parse(pid)?.find(|result| match result {
      Ok(entry) => {
        if inodes.contains(&entry.inode) {
          entry.addr == Ipv4Addr::LOCALHOST
        } else {
          false
        }
      },
      Err(_) => true,
    });
    match result {
      None => {
        if start.elapsed() >= PORT_FIND_TIMEOUT {
          bail!("failed to find local host port for process {pid}");
        }
        sleep(Duration::from_millis(1))
      },
      Some(result) => {
        break result
          .context("failed to find localhost proc tcp entry")?
          .port
      },
    }
  };

  Ok(port)
}


/// A builder for configurable launch of a Chromedriver process.
#[derive(Debug)]
pub struct Builder {
  /// The path to the `chromedriver` binary to use.
  chromedriver: PathBuf,
  /// The timeout to use waiting for `chromedriver` to start up
  /// properly.
  timeout: Duration,
}

impl Builder {
  /// Set the Chromedriver to use.
  pub fn set_chromedriver(mut self, chromedriver: impl AsRef<Path>) -> Self {
    self.chromedriver = chromedriver.as_ref().to_path_buf();
    self
  }

  /// Set the timeout to wait for Chromedriver to start up properly.
  pub fn set_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// Launch the Chromedriver process and wait for it to be fully
  /// initialized and serving a webdriver service.
  pub fn launch(self) -> Result<Chromedriver> {
    let process = Command::new(CHROME_DRIVER)
      .arg("--port=0")
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .with_context(|| format!("failed to launch `{CHROME_DRIVER}` instance"))?;

    let pid = process.id();
    let port = find_localhost_port(pid)?;

    let slf = Chromedriver { process, port };
    Ok(slf)
  }
}

impl Default for Builder {
  fn default() -> Self {
    Self {
      chromedriver: PathBuf::from(CHROME_DRIVER),
      timeout: PORT_FIND_TIMEOUT,
    }
  }
}


/// A client for shaving data of websites.
pub struct Chromedriver {
  /// The Chromdriver process.
  process: Child,
  /// The port on which the webdriver protocol is being served.
  port: u16,
}

impl Chromedriver {
  /// Launch a Chromedriver process and wait for it to be serving a
  /// webdriver service.
  pub fn launch() -> Result<Self> {
    Self::builder().launch()
  }

  /// Create a [`Builder`] for configurable launch of a Chromedriver
  /// process.
  pub fn builder() -> Builder {
    Builder::default()
  }

  /// Destroy the Chromedriver process, freeing up all resources.
  #[inline]
  fn destroy_impl(&mut self) -> Result<()> {
    self
      .process
      .kill()
      .context("failed to shut down chromedriver process")
  }

  /// Destroy the Chromedriver process, freeing up all resources.
  #[inline]
  pub fn destroy(mut self) -> Result<()> {
    self.destroy_impl()
  }

  /// Retrieve the socket address on which the webdriver service is
  /// listening.
  #[inline]
  pub fn socket_addr(&self) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), self.port)
  }
}

impl Drop for Chromedriver {
  fn drop(&mut self) {
    let _result = self.destroy_impl();
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::net::TcpListener;
  use std::process;


  /// Check that we can find a bound port on localhost.
  #[test]
  fn localhost_port_finding() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let port = find_localhost_port(process::id()).unwrap();
    assert_eq!(port, addr.port());
  }
}
