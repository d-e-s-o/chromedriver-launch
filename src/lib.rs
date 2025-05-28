// Copyright (C) 2025 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

//! A crate for launching a `chromedriver` instance on a free port and
//! retrieving said port. The crate is useful in `WebDriver` contexts,
//! i.e., anything that involves controlling a browser remotely.

mod chromedriver;
mod socket;
mod tcp;
mod util;

pub use chromedriver::Builder;
pub use chromedriver::Chromedriver;
