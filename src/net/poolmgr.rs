//! Connection Pool.  

use std::collections::RingBuf;
use std::io::{IoResult,  IoError, IoErrorKind};
use std::sync::{ Arc, Mutex};
use std::default::Default;

use net::conn;
use net::config;



pub struct ConnectionPool {
   // idle_conns:  Mutex<RingBuf<conn::Connection>>,
    idle_conns:  RingBuf<conn::Connection>,
 //   inuse_conns:  RingBuf<&'a mut conn::Connection>,
    min_conns: uint,
    max_conns: uint,
    tmp_conn_allowed: bool,
    config: config::Config,
    conns_inuse: uint,
    
}
impl  Default for ConnectionPool {
    fn default() -> ConnectionPool {
        ConnectionPool { 
          //  idle_conns: Mutex::new(RingBuf::new()),
            idle_conns: RingBuf::new(),
            min_conns: 0,
            max_conns: 10, 
            tmp_conn_allowed: true,
            config: Default::default(),
            conns_inuse: 0u,
        }
    }
}
/// Connection pool
impl  ConnectionPool {
    /// New instance
    pub fn new(pool_min_size: uint, pool_max_size: uint, tmp_allowed: bool, conn_config: &config::Config) -> ConnectionPool {
        ConnectionPool { 
           // idle_conns: Mutex::new(RingBuf::new()),
             idle_conns: RingBuf::new(),
        //    inuse_conns: RingBuf::new(),
            min_conns: pool_min_size,
            max_conns: pool_max_size, 
            tmp_conn_allowed: tmp_allowed,
            config: conn_config.clone(),
            conns_inuse: 0u,
        }
    }
    #[cfg(test)]
    pub fn idle_conns_length(& self) -> uint {
       //  let mut idleconn = self.idle_conns.lock();
       //  let mut idleconn = self.idle_conns;
         self.idle_conns.len()

    }
 
    pub fn init(&mut self) -> bool {
       //  let mut idleconn = self.idle_conns.lock();
        //  let  idleconn = self.idle_conns;
         self.idle_conns.reserve(self.max_conns);
         for i in range(0u, self.min_conns) {
            info!("Init:Creating connection {}", i);
            let  conn =   conn::Connection::connect(&self.config);
            match conn {
                Ok(c) =>  self.idle_conns.push_back(c) ,
                Err(e) => { 
                    error!("Failed to create a connection: {}", e); 
                    return false; 
                }
            }
        }
        return true;
    }
    ///releae connection
    pub fn release(&mut self, conn: conn::Connection ) {
       // let mut idleconn = self.idle_conns.lock();
       //  let  idleconn = self.idle_conns;
        let total_count = self.idle_conns.len() + self.conns_inuse;
        warn!("Total_count: {}", total_count);
        if total_count <= self.max_conns  {
            self.idle_conns.push_back(conn);
            self.conns_inuse -= 1;
        }else 
        //object goes out of scope
         {
          //  let c = conn;
          self.conns_inuse -= 1;
          warn!("It should trigger drop connection");
        }
    }

    /// Aquire Connection
    pub fn aquire(&mut self) -> IoResult<conn::Connection> {
       // let mut idleconn = self.idle_conns.lock();
        // let  idleconn = self.idle_conns;
       if !self.idle_conns.is_empty()  {
            let result = self.idle_conns.pop_front();
            if result.is_some() {
                let conn = result.unwrap();
            //    self.inuse_conns.push_back(conn);
                 self.conns_inuse += 1;
                return Ok(conn);
            }
       }
       warn!("Allocating new connection");
       let total_count = self.idle_conns.len() + self.conns_inuse;
       if total_count >= self.max_conns  && self.tmp_conn_allowed == false {
           return Err(IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "No connection available",
            detail: Some("Max pool size has reached and temporary connections are not allowed.".to_string()),
        });
           
       }
       let conn =  conn::Connection::connect(&self.config);
        match conn {
            Ok(c) => {
                self.conns_inuse += 1;
                
             //   self.inuse_conns.push_back(&c);
                return Ok(c);
            },
            Err(e) => { 
                error!("Failed to create a connection: {}", e); 
                return Err(e); 
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::default::Default;
    use std::sync::{ Arc, Mutex };
    use std::cell::RefCell;
    use std::rc::Rc;
    use net::config;
    #[test]
    fn test_new() {
        let  cfg : config::Config = Default::default();
        let mut pool = super::ConnectionPool::new(0, 5, false, &cfg);
        assert_eq!(pool.idle_conns_length(), 0);
    }

    #[test]
    fn test_init() {
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(80);
        cfg.server = Some("74.125.226.169".to_string());
        let mut pool = super::ConnectionPool::new(2, 5, false, &cfg);
        assert_eq!(pool.init(), true);
        assert_eq!(pool.idle_conns_length(), 2);
         let mut c1 = pool.aquire().unwrap();
         c1.writer.write_str("GET google.com\r\n").unwrap();
         c1.writer.flush().unwrap();
          warn!("reading_u8");
         let r = c1.reader.read_line();
         warn!("reading_u8: {}", r);

         
    }

    #[test]
    fn test_example() {
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(80);
        cfg.server = Some("google.com".to_string());
        let mut pool = super::ConnectionPool::new(2, 20, true, &cfg);
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

    #[test]
    fn test_aquire_relese() {
        info!("test_aquire_relese started---------")
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(80);
        cfg.server = Some("74.125.226.169".to_string());
        let mut pool = super::ConnectionPool::new(2, 2, true, &cfg);
        assert_eq!(pool.init(), true);
        assert_eq!(pool.idle_conns_length(), 2);
        {
        let c1 = pool.aquire().unwrap();
         assert_eq!(pool.idle_conns_length(), 1);
        let c2 = pool.aquire().unwrap();
         assert_eq!(pool.idle_conns_length(), 0);
        let c3 = pool.aquire().unwrap();
        pool.release(c1);
         assert_eq!(pool.idle_conns_length(), 0);
        pool.release(c2);
         assert_eq!(pool.idle_conns_length(), 1);
        pool.release(c3);
         assert_eq!(pool.idle_conns_length(), 2);
       }
       warn!("Only one is released");
    }

    #[test]
    fn test_aquire_relese_multithread() {
        info!("test_aquire_relese started---------")
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(80);
        cfg.server = Some("74.125.226.169".to_string());
        let mut pool = super::ConnectionPool::new(2, 20, true, &cfg);
         assert_eq!(pool.init(), true);
        let p_safe = Arc::new(Mutex::new(pool));
        for _ in range(0u, 2) {
        let p1 = p_safe.clone();
            spawn(proc() {
       
       
       // assert_eq!(pool.idle_conns_length(), 2);
        
        let c1 = p1.lock().aquire().unwrap();
        // assert_eq!(pool.idle_conns_length(), 1);
        let c2 = p1.lock().aquire().unwrap();
        // assert_eq!(pool.idle_conns_length(), 0);
        let c3 = p1.lock().aquire().unwrap();
        p1.lock().release(c1);
        // assert_eq!(pool.idle_conns_length(), 0);
        p1.lock().release(c2);
        // assert_eq!(pool.idle_conns_length(), 1);
        p1.lock().release(c3);
        // assert_eq!(pool.idle_conns_length(), 2);
       
       });
            
       }
      // assert_eq!(pool.idle_conns_length(), 2);
       warn!("Only one is released");
    }

    #[test]
    #[cfg(feature = "ssl")]
    fn test_init_ssl() {
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(443);
        cfg.server = Some("74.125.226.169".to_string());
        cfg.use_ssl = Some(true);
        let mut pool = super::ConnectionPool::new(2, 5, false, &cfg);
        assert_eq!(pool.init(), true);
    }
}