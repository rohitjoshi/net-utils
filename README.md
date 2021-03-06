net-utils
=========


[![Build Status](https://travis-ci.org/rohitjoshi/net-utils.svg?branch=master)](https://travis-ci.org/rohitjoshi/net-utils)


Provides networking utility including TCP/SSL connection and connection manager.

Here is an example of using connection pool.

Define the below dependency in Cargo.toml

    [dependencies.net-utils]
    git = "https://github.com/rohitjoshi/net-utils.git"

Here is your main.rs

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
        let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
        //get the connection
        let mut conn = pool.aquire().unwrap();
        conn.writer.write_str("GET google.com\r\n").unwrap();
        conn.writer.flush().unwrap();
        let r = conn.reader.read_line();
        println!("Received {}", r);
        pool.release(conn);
    }

Here is above example used in multi-threded environment


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
        let mut pool = poolmgr::ConnectionPool::new(2, 20, true, &cfg);
        let pool = Arc::new(Mutex::new(pool));
        for _ in range(0u, 2) {
            let pool = pool.clone();
            spawn(move || {
                let mut conn = pool.lock().aquire().unwrap();
                conn.writer.write_str("GET google.com\r\n").unwrap();
                conn.writer.flush().unwrap();
                let r = conn.reader.read_line();
                println!("Received {}", r);
                pool.lock().release(conn);
           });
        }
    }

To enable SSL connectivity,  compile using --feature ssl
e.g.  For executing SSL test cases, run
    cargo test --features ssl
    
    
## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
