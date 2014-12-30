//! Connection Pool.  

use std::collections::RingBuf;
use std::io::{IoResult,  IoError, IoErrorKind, LineBufferedWriter, stdio, stderr} ;
use std::sync::{ Mutex};
use std::sync::atomic::{AtomicUint, Ordering};
use std::default::Default;
use log::{Logger,LogRecord, set_logger};
use log;
use net::conn;
use net::config;
use time;

/// Customer Logger which supports timestamp, linenumber and file-name
 pub struct CustLogger {
      handle: LineBufferedWriter<stdio::StdWriter>,
 }

/// Logger trait Implementationf for Custom Logger
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




/// ConnectionPool which provide pooling capability for Connection objects
/// It has support for max number of connections with temporary allowable connections
pub struct ConnectionPool {
    idle_conns:  Mutex<RingBuf<conn::Connection>>,
    min_conns: uint,
    max_conns: uint,
    tmp_conn_allowed: bool,
    config: config::Config,
    conns_inuse: AtomicUint, 
}

/// Default implementation for  ConnectionPool
impl  Default for ConnectionPool {
    fn default() -> ConnectionPool {
      log::set_logger(box CustLogger { handle: stderr() } );
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

/// Connection pool implementation
impl  ConnectionPool {
    /// New instance
    pub fn new(pool_min_size: uint, pool_max_size: uint, tmp_allowed: bool, conn_config: &config::Config) -> ConnectionPool {
        log::set_logger(box CustLogger { handle: stderr() } );
        ConnectionPool { 
            idle_conns: Mutex::new(RingBuf::new()),
            min_conns: pool_min_size,
            max_conns: pool_max_size, 
            tmp_conn_allowed: tmp_allowed,
            config: conn_config.clone(),
            conns_inuse: AtomicUint::new(0u),
        }
    }
    #[cfg(test)]
    pub fn idle_conns_count(& self) -> uint {
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
    /// Release all :  Remove all connections  from th pool
    pub fn release_all(& self) {
      info!("release_all called");
      info!("It should trigger drop connection");
      self.idle_conns.lock().clear();
      self.conns_inuse.store(0, Ordering::Relaxed);
      let total_count = self.idle_conns.lock().len() + self.conns_inuse.load(Ordering::Relaxed);
      info!("release_all called: Total_count: {}", total_count);
    }


    ///Releae connection
    #[allow(dead_code)]
    pub fn release(& self, conn: conn::Connection ) {
 
        let total_count = self.idle_conns.lock().len();
        info!("release(): idle connection: {}", total_count);
        if total_count < self.max_conns && conn.is_valid()  {
            info!("Pushing back to ideal_conns");
            self.idle_conns.lock().push_back(conn);
            self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
        }else if  !conn.is_valid() {
         //    let c = conn;
           self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
           info!("Connection not valid. It should trigger drop connection");
        }
        else 
        //object goes out of scope
         {
          //  let c = conn;
           self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
           info!("It should trigger drop connection");
        }
       
        info!("release() end: Total_count: {}", self.idle_conns.lock().len() + self.conns_inuse.load(Ordering::Relaxed));
      
    }

    /// Drop connection.  Use only if discconect.
    #[allow(unused_variables)]
    pub fn drop(& self, conn: conn::Connection ) {
        self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
        warn!("drop() end: Total_count: {}", self.idle_conns.lock().len() + self.conns_inuse.load(Ordering::Relaxed));
        
    }


    /// Aquire Connection
    pub fn acquire(& self) -> IoResult<conn::Connection> {
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
                error!("Failed to create a connection : {}", e); 
                return Err(e); 
            }
        }
    
  }
}

#[cfg(test)]
#[allow(experimental)]
pub mod test {
    use std::io::{TcpListener, Listener,Acceptor,TcpStream, stderr};
    use std::default::Default;
    use std::sync::{ Arc,  };
    use std::thread::Thread;
    use net::config;
    use std::io;
    use std::io::test;
    use log;
    use std::comm;
    use std::io::timer::sleep;
    use std::time::duration::Duration;
    #[cfg(test)]
    #[allow(unused_variables)]
    pub fn listen_ip4_localhost(port: u16, rx:Receiver<int>)  {
        let uri = format!("127.0.0.1:{}", port);
        let mut acceptor = TcpListener::bind(uri.as_slice()).listen().unwrap();
       // let mut acceptor = listener.listen();
       acceptor.set_timeout(Some(250));
        for  stream in acceptor.incoming() {
         
           match stream {
              Err(e) => debug!("Accept err {}", e),
              Ok(stream) => {
                 info!("Got new connection on port {}", port);
                  Thread::spawn(move || {
                    info!("{}", handle_client(stream));
                  }).detach();
              }
            }
            match rx.try_recv() {
                Ok(_) | Err(comm::Disconnected) => {
                    info!("******Terminating thread with port {}." , port);
                    break;
                }
                Err(comm::Empty) => {}
            }
        }
        drop(acceptor);
    }
    #[cfg(test)]
    #[allow(unused_variables)]
    fn handle_client(mut stream: io::TcpStream) -> io::IoResult<()>{

           let mut buf = [0];
           loop {
               let got = try!(stream.read(&mut buf));
                if got == 0 {
                   warn!("handle_client: Received: 0");
                   let result = stream.write_str("Fail to read\r\n");
                   stream.flush().unwrap();
                   break;
                }else {
                    warn!("handle_client: Received: {}. Sending it back", buf);
                    let result = stream.write(buf.slice(0, got));
                   
                  break;
                }
            }
            Ok(())
      }
        

    #[test]
    fn test_new() {
      info!("test_new started---------");
      log::set_logger(box super::CustLogger { handle: stderr() } );
        let  cfg : config::Config = Default::default();
        let  pool = super::ConnectionPool::new(0, 5, false, &cfg);
        assert_eq!(pool.idle_conns_count(), 0);
        sleep(Duration::milliseconds(1000));
        pool.release_all();
        assert_eq!(pool.idle_conns_count(), 0);
        info!("test_new ended---------");
    }

    ///test google
     #[test]
    fn test_google() {
        log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_lib started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(80); //Some(io::test::next_test_port());
        cfg.server = Some("google.com".to_string()); //Some("127.0.0.1".to_string());
 
        let  pool = super::ConnectionPool::new(2, 20, true, &cfg);
         assert_eq!(pool.init(), true);
        assert_eq!(pool.idle_conns_count(), 2);
        let mut conn = pool.acquire().unwrap();
        assert_eq!(conn.is_valid(), true);
         assert_eq!(pool.idle_conns_count(), 1);
        conn.writer.write_str("GET google.com\r\n").unwrap();
        conn.writer.flush().unwrap();
        let r = conn.reader.read_line();
        println!("Received {}", r);
        pool.release(conn);
        assert_eq!(pool.idle_conns_count(), 2);
        info!("test_lib ended---------");

    }

    #[test]
    fn test_init() {
        log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_init started---------");
        
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();

       

        {
            info!("test_init starting channel---------");
           let (tx, rx): (Sender<int>, Receiver<int>) = channel();
           info!("test_init spawning thread---------");
           Thread::spawn(move || {
              info!("test_init calling listen_Ip4_localhost with port {}", listen_port);
              listen_ip4_localhost(listen_port, rx);
          }).detach();
          sleep(Duration::milliseconds(500));
          info!("test_init starting connection pool");
          let  pool = super::ConnectionPool::new(1, 5, false, &cfg);
          assert_eq!(pool.init(), true);
          assert_eq!(pool.idle_conns_count(), 1);
          let mut c1 = pool.acquire().unwrap();
           info!("test_init acquire connection");
          assert_eq!(c1.is_valid(), true);
           info!("test_init send data");
          c1.writer.write_str("GET google.com\r\n").unwrap();
          c1.writer.flush().unwrap();       
          info!("reading line");
          let r = c1.reader.read_line();
          info!("reading received: {}", r);
           info!("sending 0 to channel");
          tx.send(0);
          info!("releasing all");
          pool.release_all();
          assert_eq!(pool.idle_conns_count(), 0);
          
        }
       
        info!("test_init ended---------");
         
    }

    #[test]
    fn test_example() {
      log::set_logger(box super::CustLogger { handle: stderr() } );
      info!("test_example started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<int>, Receiver<int>) = channel();
        Thread::spawn(move || {
          listen_ip4_localhost(listen_port, rx);
        }).detach();
        let  pool = super::ConnectionPool::new(2, 5, true, &cfg);
        let pool_shared = Arc::new(pool);
        for _ in range(0u, 6) {
            let pool = pool_shared.clone();
            Thread::spawn(move || {
                let mut conn = pool.acquire().unwrap();

                conn.writer.write_str("GET google.com\r\n").unwrap();
                conn.writer.flush().unwrap();
                let r = conn.reader.read_line();
                println!("Received {}", r);
                pool.release(conn);
           }).detach();
        }
         sleep(Duration::milliseconds(1000));
         assert_eq!(pool_shared.idle_conns_count(), 5);
         pool_shared.release_all();
         assert_eq!(pool_shared.idle_conns_count(), 0);
         tx.send(0);
        info!("test_example ended---------");
    }

    #[test]
    fn test_acquire_release_1() {
      log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_acquire_release started---------");
        let mut cfg : config::Config = Default::default();

        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<int>, Receiver<int>) = channel();
        Thread::spawn(move || {
          listen_ip4_localhost(listen_port, rx);
        }).detach();
        {
          let  pool = super::ConnectionPool::new(2, 2, true, &cfg);
          assert_eq!(pool.init(), true);
          assert_eq!(pool.idle_conns_count(), 2);
        
          let c1 = pool.acquire().unwrap();
          assert_eq!(pool.idle_conns_count(), 1);
          let c2 = pool.acquire().unwrap();
          assert_eq!(pool.idle_conns_count(), 0);
          let c3 = pool.acquire().unwrap();
          pool.release(c1);
          assert_eq!(pool.idle_conns_count(), 1);
          pool.release(c2);
          assert_eq!(pool.idle_conns_count(), 2);
          pool.release(c3);
          assert_eq!(pool.idle_conns_count(), 2);
           
          pool.release_all();
          assert_eq!(pool.idle_conns_count(), 0);
          tx.send(0);
       }
       
       info!("test_acquire_release ended---------");
    }

    #[test]
    fn test_acquire_release_multithread() {
      log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_acquire_release_multithread started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<int>, Receiver<int>) = channel();
        {
          Thread::spawn(move || {
            listen_ip4_localhost(listen_port, rx);
          }).detach();
          let  pool = super::ConnectionPool::new(2, 10, true, &cfg);
          assert_eq!(pool.init(), true);
          let pool_shared = Arc::new(pool);
          for _ in range(0u, 10) {
              let p1 = pool_shared.clone();
              Thread::spawn(move || {
                let c1 = p1.acquire().unwrap();
                let c2 = p1.acquire().unwrap();
                let c3 = p1.acquire().unwrap();
                p1.release(c1);
                p1.release(c2);
                p1.release(c3);
              }).detach();  
          }
          sleep(Duration::milliseconds(1000));
          assert_eq!(pool_shared.idle_conns_count(), 10);
          pool_shared.release_all();
          assert_eq!(pool_shared.idle_conns_count(), 0);
          tx.send(0);
        }
       
      // assert_eq!(pool_shared.idle_conns_count(), 10);
       info!("test_acquire_release_multithread ended---------");
    }

    #[test]
    fn test_acquire_release_multithread_2() {
        log::set_logger(box super::CustLogger { handle: stderr() } );
        info!("test_acquire_release_multithread_2 started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(test::next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<int>, Receiver<int>) = channel();
        Thread::spawn(move || {
          listen_ip4_localhost(listen_port, rx);
        }).detach();
        
        let  pool = super::ConnectionPool::new(2, 3, true, &cfg);
        assert_eq!(pool.init(), true);
        let pool_shared = Arc::new(pool);
        for _ in range(0u, 2) {
            let p1 =  pool_shared.clone();
            Thread::spawn(move || {  
                info!("test_acquire_release_multithread_2 acquired connection in thread");        
                let c1 = p1.acquire().unwrap();
                 info!("test_acquire_release_multithread_2 release connection in thread"); 
                p1.release(c1);

            }).detach();
       }
       sleep(Duration::milliseconds(1000));
        info!("test_acquire_release_multithread_2 out of for loop :{}", pool_shared.idle_conns_count()); 
       assert_eq!(pool_shared.idle_conns_count(), 2);
       pool_shared.release_all();
       assert_eq!(pool_shared.idle_conns_count(), 0);
       tx.send(0);
     
      info!("test_acquire_release_multithread_2 ended---------");
    }

    #[test]
    #[cfg(feature = "ssl")]
    fn test_init_ssl() {
      log::set_logger(box super::CustLogger { handle: stderr() } );
          info!("test_init_ssl started---------");
        let mut cfg : config::Config = Default::default();
        cfg.port= Some(443);
        cfg.server = Some("google.com".to_string());
        
        cfg.use_ssl = Some(true);
        let  pool = super::ConnectionPool::new(2, 5, false, &cfg);
        assert_eq!(pool.init(), true);
       
        pool.release_all();
        assert_eq!(pool.idle_conns_count(), 0);
         info!("test_init_ssl ended---------");
    }
}