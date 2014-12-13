net-utils
=========

Provides networking utility including TCP/SSL connection and connection manager.

Here is an example of using connection pool.

	use std::default::Default;
	use net::config;
	use net::poolmgr;
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

Here is above example used in multi-threded enviornment

        
    use std::default::Default;
    use std::sync::{ Arc, Mutex };
    use net::config;
    use net::poolmgr;
    	 
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
