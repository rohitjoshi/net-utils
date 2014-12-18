//! A simple, connection pool library.
#![crate_name = "net-utils"]
#![crate_type = "lib"]
#![unstable]
#![warn(missing_docs)]
#![feature(slicing_syntax,phase)]
#[cfg(feature = "ssl")] extern crate openssl;
#[phase(plugin, link)]extern crate log;
#[phase(plugin, link)]extern crate time;
pub mod net;


#[cfg(test)]
pub mod test {
	use std::default::Default;
	use net::config;
	use net::poolmgr;
    use net::poolmgr::test;
    use std::io;
    use std::io::{TcpListener, Listener,Acceptor,TcpStream, stderr};
    use log;
#[test]
fn test_lib() {
    //log::set_logger(box poolmgr::CustLogger { handle: stderr() } );
    info!("test_lib started---------");
    let mut cfg : config::Config = Default::default();
    cfg.port= Some(io::test::next_test_port());
    cfg.server = Some("127.0.0.1".to_string());
    let listen_port = cfg.port.unwrap();
        spawn(move || {
          test::listen_ip4_localhost(listen_port);
    });
   // cfg.use_ssl = Some(true);
    let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
   //get the connection
    let mut conn = pool.aquire().unwrap();
     assert_eq!(conn.is_valid(), true);
    conn.writer.write_str("GET google.com\r\n").unwrap();
    conn.writer.flush().unwrap();
    let r = conn.reader.read_line();
    println!("Received {}", r);
    pool.release(conn);
    pool.release_all();
    assert_eq!(pool.idle_conns_length(), 0);
    info!("test_lib ended---------");

}
#[test]
#[cfg(feature = "ssl")]
fn test_lib_ssl() {
    //log::set_logger(box poolmgr::CustLogger { handle: stderr() } );
    info!("test_lib_ssl started---------");
    let mut cfg : config::Config = Default::default();
    //set port to 80
    cfg.port= Some(443);
    //set host to
    cfg.server = Some("google.com".to_string());
    cfg.use_ssl = Some(true);
    let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
   //get the connection
    let mut conn = pool.aquire().unwrap();
     assert_eq!(conn.is_valid(), true);
    conn.writer.write_str("GET google.com\r\n").unwrap();
    conn.writer.flush().unwrap();
    let r = conn.reader.read_line();
    debug!("SSL Received {}", r);
    pool.release(conn);
    pool.release_all();
    assert_eq!(pool.idle_conns_length(), 0);
     info!("test_lib_ssl ended---------");
}
}


