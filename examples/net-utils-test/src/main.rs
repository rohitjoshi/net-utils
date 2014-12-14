extern crate "net-utils" as utils;
use std::default::Default;
use std::sync::{ Arc, Mutex };


use utils::net::config;
use utils::net::poolmgr;


fn main() {
    let mut cfg : config::Config = Default::default();
    //set port to 80
    cfg.port= Some(80);
    //set host to
    cfg.server = Some("google.com".to_string());
   // cfg.use_ssl = Some(true);
    let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
    let pool = Arc::new(Mutex::new(pool));
    for _ in range(0u, 2) {
        let pool = pool.clone();
        spawn(proc() {
            let mut conn = pool.lock().aquire().unwrap();
            conn.writer.write_str("GET google.com\r\n").unwrap();
            conn.writer.flush().unwrap();
            let r = conn.reader.read_line();
            println!("Received {}", r);
            pool.lock().release(conn);
       });
    }
}