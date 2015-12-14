use std::io::prelude::*;
use std::default::Default;
use std::sync::{ Arc };
use std::thread;
use std::thread::sleep;
//use std::io::timer::sleep;
use std::time::Duration;
extern crate net_utils as utils;
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
    for _ in 0u32..2 {
            let pool = pool_shared.clone();
             thread::spawn(move || {
                let mut conn = pool.acquire().unwrap();
                println!("Sending request: GET google.com\r\n");
                conn.writer.write("GET google.com\r\n".as_bytes()).unwrap();
                conn.writer.flush().unwrap();
                let mut buffer = String::new();
                let r = conn.reader.read_line(&mut buffer);
                if r.unwrap() > 0 {
                  println!("Received {}", buffer);
                }
                pool.release(conn);
           });

    }
    sleep(Duration::from_millis(1000));


}
