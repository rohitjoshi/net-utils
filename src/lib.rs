// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Net-utils library provides a configurable TCP/SSL client connection pool
//! A simple, connection pool library.
#![crate_name = "net_utils"]
#![crate_type = "lib"]
#![warn(missing_docs)]
// #![allow(unstable)]
#![allow(unused_must_use)]
#[cfg(feature = "ssl")]
extern crate openssl;
#[macro_use]
extern crate log;
extern crate time;
extern crate uuid;
/// module net provides the TCP/SSL connection and connection pool functionality
pub mod net;
