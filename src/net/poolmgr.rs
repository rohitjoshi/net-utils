//! Connection Pool.  

use std::collections::RingBuf;
use std::io::{IoResult,  IoError, IoErrorKind, LineBufferedWriter, stdio, stderr} ;
use std::sync::{ Arc, Mutex};
use std::sync::atomic::{AtomicUint, Ordering};
use std::default::Default;
use log::{Logger,LogRecord,LogLevel,LogLocation, set_logger};
use log;
use net::conn;
use net::config;
use time;


 pub struct CustLogger {
     pub handle: LineBufferedWriter<stdio::StdWriter>,
 }

impl Logger  for CustLogger {
     fn log(&mut self, record: &LogRecord) {
         match writeln!(&mut self.handle,
                        "{}:{}:{}:{}:{} {}",
                        time::strftime("%Y-%m-%d %H:%M:%S %Z", &time::now()).unwrap(),
                        record.level,
                        record.module_path,
                        record.file,
                        record.line,
                        record.args) {
             Err(e) => panic!("failed to log: {}", e),
             Ok(()) => {}
         }
     }
 }




//#[deriving(Send, Sync)]
pub struct ConnectionPool {
    idle_conns:  Mutex<RingBuf<conn::Connection>>,
   // idle_conns:  RingBuf<conn::Connection>,
 //   inuse_conns:  RingBuf<&'a mut conn::Connection>,
    min_conns: uint,
    max_conns: uint,
    tmp_conn_allowed: bool,
    config: config::Config,
    conns_inuse: AtomicUint, 
}
impl  Default for ConnectionPool {
    fn default() -> ConnectionPool {
      //log::set_logger(box CustLogger { handle: stderr() } );
        ConnectionPool { 
            idle_conns: Mutex::new(RingBuf::new()),
         //   idle_conns: RingBuf::new(),
            min_conns: 0,
            max_conns: 10, 
            tmp_conn_allowed: true,
            config: Default::default(),
            conns_inuse: AtomicUint::new(0u),
        }
    }
}
/// Connection pool
impl  ConnectionPool {
    /// New instance
    pub fn new(pool_min_size: uint, pool_max_size: uint, tmp_allowed: bool, conn_config: &config::Config) -> ConnectionPool {
        //log::set_logger(box CustLogger { handle: stderr() } );
        ConnectionPool { 
            idle_conns: Mutex::new(RingBuf::new()),
           //  idle_conns: RingBuf::new(),
        //    inuse_conns: RingBuf::new(),
            min_conns: pool_min_size,
            max_conns: pool_max_size, 
            tmp_conn_allowed: tmp_allowed,
            config: conn_config.clone(),
            conns_inuse: AtomicUint::new(0u),
        }
    }
    #[cfg(test)]
    pub fn idle_conns_length(& self) -> uint {
       //  let mut idleconn = self.idle_conns.lock().lock();
       //  let mut idleconn = self.idle_conns;
         self.idle_conns.lock().len()

    }
    /// Initial the connection pool
    pub fn init(& self) -> bool {
       //  let mut idleconn = self.idle_conns.lock().lock();
        //  let  idleconn = self.idle_conns;
         self.idle_conns.lock().reserve(self.max_conns);
         for i in range(0u, self.min_conns) {
            info!("Init:Creating connection {}", i);
            let  conn =   conn::Connection::connect(&self.config);
            match conn {
                Ok(c) =>  self.idle_conns.lock().push_back(c) ,
                Err(e) => { 
                    error!("Failed to create a connection: {}", e); 
                    return false; 
                }
            }
        }
        return true;
    }

    pub fn release_all(& self) {
      self.idle_conns.lock().clear();
      self.conns_inuse.store(0, Ordering::Relaxed);
    }


    ///Releae connection
    pub fn release(& self, conn: conn::Connection ) {
 
        let total_count = self.idle_conns.lock().len() + self.conns_inuse.load(Ordering::Relaxed);
        warn!("release(): Total_count: {}", total_count);
        if total_count <= self.max_conns && conn.is_valid() {
            self.idle_conns.lock().push_back(conn);
            self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
        }else 
        //object goes out of scope
         {
            let c = conn;
           self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
           info!("It should trigger drop connection");
        }
       
        warn!("release() end: Total_count: {}", self.idle_conns.lock().len() + self.conns_inuse.load(Ordering::Relaxed));
      
    }

    ///Drop connection.  Use only if discconect.
    pub fn drop(& self, conn: conn::Connection ) {
         
         self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
         {let c = conn;}
          warn!("drop() end: Total_count: {}", self.idle_conns.lock().len() + self.conns_inuse.load(Ordering::Relaxed));
        
    }


    /// Aquire Connection
    pub fn aquire(& self) -> IoResult<conn::Connection> {
       // let mut idleconn = self.idle_conns.lock().lock();
        // let  idleconn = self.idle_conns;

         let mut conns = self.idle_conns.lock();
         {
            if !conns.is_empty()  {
              let result = conns.pop_front();
              if result.is_some() {
                let conn = result.unwrap();
            //    self.inuse_conns.push_back(conn);
                 self.conns_inuse.fetch_add(1, Ordering::Relaxed);
                return Ok(conn);
              }
        }
       debug!("Allocating new connection");
       let total_count = conns.len() + self.conns_inuse.load(Ordering::Relaxed);
       if total_count >= self.max_conns  && self.tmp_conn_allowed == false {
           return Err(IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "No connection available",
            detail: Some("Max pool size has reached and temporary connections are not allowed.".to_string()),
        });
           
       }
     }
       let conn =  conn::Connection::connect(&self.config);
        match conn {
            Ok(c) => {
               self.conns_inuse.fetch_add(1, Ordering::Relaxed);
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
#[allow(experimental)]
pub mod test {
    use std::io;
    use std::os;
    use std::io::{TcpListener, Listener,Acceptor,TcpStream, stderr};
    use std::default::Default;
    use std::sync::{ Arc, Mutex };
    use std::cell::RefCell;
    use std::cell::UnsafeCell;
    use std::rc::Rc;
    use net::config;
    use std::io::test;
    use log;
    #[cfg(test)]
    pub fn listen_ip4_localhost(port: u16)  {
        let uri = format!("127.0.0.1:{}", port);
        let mut acceptor = TcpListener::bind(uri.as_slice()).listen().unwrap();
       // let mut acceptor = listener.listen();
        for  stream in acceptor.incoming() {
           match stream {
            Err(e) => warn!("Accept err {}", e),
            Ok(stream) => {
                spawn(move || {
                    debug!("{}", handle_client(stream));
                   
                })
            }
            }
        }
        drop(acceptor);
    }
    #[cfg(test)]
    fn handle_client(mut stream: io::TcpStream) -> io::IoResult<()>{

           let mut buf = [0];
           loop {
               let got = try!(stream.read(&mut buf));
                if got == 0 {
                   // Is it possible? Or IoError will be raised anyway?
                 //  break
                }else {
                warn!("Test Server: Received: {}. Sending it back", buf);
               stream.write(buf.slice(0, got));
               break;
             }
         }
        Ok(())
      }
        

    #[test]
    fn test_new() {
      info!("test_new started---------");
      //log::set_logger(box super::CustLogger { handle: stderr() } );
        let  cfg : config::Config = Default::default();
        let mut pool = super::ConnectionPool::new(0, 5, false, &cfg);
        assert_eq!(pool.idle_conns_length(), 0);
        pool.release_all();
        assert_eq!(pool.idle_conns_length(), 0);
        info!("test_new ended---------");
    }

    #[test]
    fn test_init() {
      //log::set_logger(box super::CustLogger { handle: stderr() } );
       info!("test_init started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        spawn(move || {
          listen_ip4_localhost(listen_port);
        });
        let mut pool = super::ConnectionPool::new(1, 5, false, &cfg);
        assert_eq!(pool.init(), true);
        assert_eq!(pool.idle_conns_length(), 1);
         let mut c1 = pool.aquire().unwrap();
         assert_eq!(c1.is_valid(), true);
         c1.writer.write_str("GET google.com\r\n").unwrap();
         c1.writer.flush().unwrap();
          debug!("reading_u8");
         let r = c1.reader.read_line();
         debug!("reading_u8: {}", r);
          pool.release_all();
          assert_eq!(pool.idle_conns_length(), 0);
         info!("test_init ended---------");
         
    }

    #[test]
    fn test_example() {
      //log::set_logger(box super::CustLogger { handle: stderr() } );
      info!("test_example started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        spawn(move || {
          listen_ip4_localhost(listen_port);
        });
        let  pool = super::ConnectionPool::new(2, 20, true, &cfg);
        let pool_shared = Arc::new(pool);
        for _ in range(0u, 2) {
            let pool = pool_shared.clone();
            spawn(move || {
                let mut conn = pool.aquire().unwrap();

                conn.writer.write_str("GET google.com\r\n").unwrap();
                conn.writer.flush().unwrap();
                let r = conn.reader.read_line();
                println!("Received {}", r);
                pool.release(conn);
           });
        }
         pool_shared.release_all();
         assert_eq!(pool_shared.idle_conns_length(), 0);
        info!("test_example ended---------");
    }

    #[test]
    fn test_aquire_relese() {
      //log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_aquire_relese started---------");
        let mut cfg : config::Config = Default::default();

        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        spawn(move || {
          listen_ip4_localhost(listen_port);
        });
        let  pool = super::ConnectionPool::new(2, 2, true, &cfg);
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
         pool.release_all();
        assert_eq!(pool.idle_conns_length(), 0);
       }
       info!("test_aquire_relese ended---------");
    }

    #[test]
    fn test_aquire_relese_multithread() {
      //log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_aquire_relese_multithread started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        spawn(move || {
          listen_ip4_localhost(listen_port);
        });
        let  pool = super::ConnectionPool::new(2, 10, true, &cfg);
         assert_eq!(pool.init(), true);
        let pool_shared = Arc::new(pool);
        for _ in range(0u, 10) {
        let p1 = pool_shared.clone();
            spawn(move || {
              let c1 = p1.aquire().unwrap();
              let c2 = p1.aquire().unwrap();
              let c3 = p1.aquire().unwrap();
              p1.release(c1);
              p1.release(c2);
              p1.release(c3);
        });  
       }
       pool_shared.release_all();
       assert_eq!(pool_shared.idle_conns_length(), 0);
      // assert_eq!(pool_shared.idle_conns_length(), 10);
       info!("test_aquire_relese_multithread ended---------");
    }

    #[test]
    fn test_aquire_relese_multithread_2() {
      //log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_aquire_relese_multithread_2 started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        spawn(move || {
          listen_ip4_localhost(listen_port);
        });
        
        let  pool = super::ConnectionPool::new(2, 2, true, &cfg);
        assert_eq!(pool.init(), true);
        let pool_shared = Arc::new(pool);
        for _ in range(0u, 2) {
            let p1 =  pool_shared.clone();
            spawn(move || {          
                let c1 = p1.aquire().unwrap();
                p1.release(c1);

            });
       }
       pool_shared.release_all();
     
       assert_eq!(pool_shared.idle_conns_length(), 0);
      // assert_eq!(pool.idle_conns_length(), 2);
       
      info!("test_aquire_relese_multithread_2 ended---------");
    }

    #[test]
    #[cfg(feature = "ssl")]
    fn test_init_ssl() {
      //log::set_logger(box super::CustLogger { handle: stderr() } );
          info!("test_init_ssl started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(443);
        cfg.server = Some("google.com".to_string());
        
        cfg.use_ssl = Some(true);
        let  pool = super::ConnectionPool::new(2, 5, false, &cfg);
        assert_eq!(pool.init(), true);
        pool.release_all();
        assert_eq!(pool.idle_conns_length(), 0);
         info!("test_init_ssl ended---------");
    }
}