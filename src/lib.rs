//! A simple, connection pool library.
#![crate_name = "net-utils"]
#![crate_type = "lib"]
#![unstable]
#![warn(missing_docs)]
#![feature(slicing_syntax,phase)]
#[cfg(feature = "ssl")] extern crate openssl;
#[phase(plugin, link)]extern crate log;
#[phase(plugin, link)]extern crate time;
/// module net provides the TCP/SSL connection and connection pool functionality
pub mod net;

