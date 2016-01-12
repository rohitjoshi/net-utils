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
#![crate_name = "net_utils"]
#![crate_type = "lib"]
#![warn(missing_docs)]
// #![allow(unstable)]
#![allow(unused_must_use)]

/// # Example
///
///Define the below dependency in Cargo.toml
///
///[dependencies.net-utils]
///git = "https://github.com/rohitjoshi/net-utils.git"
///
/// ```
/// extern crate "net-utils" as utils;
/// use std::default::Default;
/// use std::sync::{ Arc, Mutex };
/// use utils::net::config;
/// use utils::net::poolmgr;

/// fn main() {
///     let mut cfg : config::Config = Default::default();
///     //set port to 80
///     cfg.port= Some(80);
///     //set host to
///     cfg.server = Some("google.com".to_string());
///     let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
///     //get the connection
///     let mut conn = pool.aquire().unwrap();
///     conn.writer.write_str("GET google.com\r\n").unwrap();
///     conn.writer.flush().unwrap();
///     let r = conn.reader.read_line();
///     println!("Received {}", r);
///     pool.release(conn);
///}
/// ```
///
///Here is above example used in multi-threded environment
/// ```
/// extern crate "net-utils" as utils;
/// use std::default::Default;
/// use std::sync::{ Arc, Mutex };
/// use utils::net::config;
/// use utils::net::poolmgr;
///
/// fn main() {
///     let mut cfg : config::Config = Default::default();
///     //set port to 80
///     cfg.port= Some(80);
///     //set host to
///     cfg.server = Some("google.com".to_string());
///     let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
///     let pool = Arc::new(Mutex::new(pool));
///     for _ in range(0u, 2) {
///         let pool = pool.clone();
///         spawn(move || {
///             let mut conn = pool.lock().aquire().unwrap();
///             conn.writer.write_str("GET google.com\r\n").unwrap();
///             conn.writer.flush().unwrap();
///             let r = conn.reader.read_line();
///             println!("Received {}", r);
///             pool.lock().release(conn);
///        });
///     }
/// }
//# ```
#[cfg(feature = "ssl")]
extern crate openssl;
#[macro_use]
extern crate log;
#[macro_use]
extern crate time;
#[macro_use]
extern crate uuid;
/// module net provides the TCP/SSL connection and connection pool functionality
pub mod net;
