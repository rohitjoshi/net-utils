extern crate "net-utils" as utils;
use std::default::Default;
use std::sync::{ Arc };
use std::thread::Thread;
use std::io::timer::sleep;
use std::time::duration::Duration;
use utils::net::config;
use utils::net::poolmgr;


fn main() {
    let mut cfg : config::Config = Default::default();
    //set port to 80
    cfg.port= Some(80);
    //set host to
    cfg.server = Some("google.com".to_string());
   // cfg.use_ssl = Some(true);
    let  pool = poolmgr::ConnectionPool::new(2, 5, true, &cfg);
    let pool_shared = Arc::new(pool);
    for _ in range(0u, 2) {
            let pool = pool_shared.clone();
            let r = Thread::spawn(move || {
                let mut conn = pool.acquire().unwrap();
                println!("Sending request: GET google.com\r\n");
                conn.writer.write_str("GET google.com\r\n").unwrap();
                conn.writer.flush().unwrap();
                let r = conn.reader.read_line();
                println!("Received {}", r.unwrap());
                pool.release(conn);
           });

    }
    sleep(Duration::milliseconds(1000));

   
}
