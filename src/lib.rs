//! A simple, connection pool library.
#![crate_name = "netutils"]
#![crate_type = "lib"]
#![unstable]
#![warn(missing_docs)]
#![feature(slicing_syntax,phase)]
#[cfg(feature = "ssl")] extern crate openssl;
#[phase(plugin, link)]extern crate log;

pub mod net;


#[cfg(test)]
pub mod test {
	use std::default::Default;
	use net::config;
	use net::poolmgr;
#[test]
fn test_lib() {
let mut cfg : config::Config = Default::default();
    //set port to 80
    cfg.port= Some(80);
    //set host to
    cfg.server = Some("google.com".to_string());
   // cfg.use_ssl = Some(true);
    let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
   //get the connection
    let mut conn = pool.aquire().unwrap();
    conn.writer.write_str("GET google.com\r\n").unwrap();
    conn.writer.flush().unwrap();
    let r = conn.reader.read_line();
    println!("Received {}", r);
    pool.release(conn);
}
#[test]
#[cfg(feature = "ssl")]
fn test_lib_ssl() {
let mut cfg : config::Config = Default::default();
    //set port to 80
    cfg.port= Some(443);
    //set host to
    cfg.server = Some("google.com".to_string());
    cfg.use_ssl = Some(true);
    let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
   //get the connection
    let mut conn = pool.aquire().unwrap();
    conn.writer.write_str("GET google.com\r\n").unwrap();
    conn.writer.flush().unwrap();
    let r = conn.reader.read_line();
    warn!("SSL Received {}", r);
    pool.release(conn);
}
}


