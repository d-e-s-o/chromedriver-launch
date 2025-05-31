// Copyright (C) 2025 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::CString;
use std::fs::read_dir;
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStringExt as _;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;

use libc::mode_t as Mode;
use libc::stat64 as Stat64;
use libc::stat64;
use libc::S_IFMT;
use libc::S_IFSOCK;

use crate::util::check;


fn stat<P>(path: P) -> Result<Stat64>
where
  P: AsRef<Path>,
{
  let mut buf = MaybeUninit::<Stat64>::uninit();
  let path = path.as_ref();
  let path_buf = path.as_os_str().to_os_string().into_vec();
  // We need to ensure NUL termination when performing the system call.
  let file = unsafe { CString::from_vec_unchecked(path_buf) };
  let result = unsafe { stat64(file.as_ptr(), buf.as_mut_ptr()) };

  check(result, -1).with_context(|| format!("stat64 failed for `{}`", path.display()))?;

  Ok(unsafe { buf.assume_init() })
}

fn is_socket(mode: Mode) -> bool {
  mode & S_IFMT == S_IFSOCK
}

fn check_socket<P>(path: P) -> Result<u64>
where
  P: AsRef<Path>,
{
  let path = path.as_ref();
  let buf = stat(path)?;

  if is_socket(buf.st_mode) {
    #[allow(trivial_numeric_casts)]
    Ok(buf.st_ino as _)
  } else {
    Err(io::Error::new(io::ErrorKind::NotFound, "no socket found"))
      .with_context(|| format!("file `{}` is not a socket", path.display()))
  }
}

/// Find all inodes of socket file descriptors opened by the given
/// process.
pub(crate) fn socket_inodes(pid: u32) -> Result<impl Iterator<Item = Result<u64>>> {
  let path = PathBuf::from(format!("/proc/{pid}/fd"));

  read_dir(&path)
    .with_context(|| format!("failed to read directory `{}`", path.display()))
    .map(move |x| {
      x.filter_map(move |entry| match entry {
        Ok(entry) => {
          let path = entry.path();
          if let Ok(inode) = check_socket(path) {
            Some(Ok(inode))
          } else {
            None
          }
        },
        Err(err) => Some(
          Err(err)
            .with_context(|| format!("failed to read directory entry below `{}`", path.display())),
        ),
      })
    })
}
