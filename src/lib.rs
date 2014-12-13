//! A simple, connection pool library.
#![crate_name = "netutils"]
#![crate_type = "lib"]
#![unstable]
#![warn(missing_docs)]
#![feature(slicing_syntax,phase)]
#[cfg(feature = "ssl")] extern crate openssl;
#[phase(plugin, link)]extern crate log;

pub mod net;


