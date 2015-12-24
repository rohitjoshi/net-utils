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
#[macro_use]
extern crate time;
#[macro_use]
extern crate uuid;
/// module net provides the TCP/SSL connection and connection pool functionality
pub mod net;
