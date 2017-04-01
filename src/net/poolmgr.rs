//! Connection Pool.

use std::collections::VecDeque;
use std::io::{Result, Error, ErrorKind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::default::Default;


use net::conn;
use net::config;


/// ConnectionPool which provide pooling capability for Connection objects
/// It has support for max number of connections with temporary allowable connections
pub struct ConnectionPool {
    idle_conns: Mutex<VecDeque<conn::Connection>>,
    min_conns: usize,
    max_conns: usize,
    tmp_conn_allowed: bool,
    config: config::Config,
    conns_inuse: AtomicUsize,
}

/// Default implementation for  ConnectionPool
impl Default for ConnectionPool {
    fn default() -> ConnectionPool {

        ConnectionPool {
            idle_conns: Mutex::new(VecDeque::new()),
            // idle_conns: VecDeque::new(),
            min_conns: 0,
            max_conns: 10,
            tmp_conn_allowed: true,
            config: Default::default(),
            conns_inuse: AtomicUsize::new(0),
        }
    }
}

/// Connection pool implementation
impl ConnectionPool {
    /// New instance
    pub fn new(pool_min_size: usize,
               pool_max_size: usize,
               tmp_allowed: bool,
               conn_config: &config::Config)
               -> ConnectionPool {
        ConnectionPool {
            idle_conns: Mutex::new(VecDeque::new()),
            min_conns: pool_min_size,
            max_conns: pool_max_size,
            tmp_conn_allowed: tmp_allowed,
            config: conn_config.clone(),
            conns_inuse: AtomicUsize::new(0),
        }
    }
    #[cfg(test)]
    pub fn idle_conns_count(&self) -> usize {
        self.idle_conns.lock().unwrap().len()

    }
    /// Initial the connection pool
    pub fn init(&self) -> bool {
        self.idle_conns.lock().unwrap().reserve(self.max_conns);
        for i in 0..self.min_conns {
            info!("*****Init:Creating connection {}", i);
            let  conn = conn::Connection::connect(&self.config);

            let host: &str = &self.config.server.clone().unwrap();
            let port = &self.config.port.unwrap();
           
            match conn {
                Ok(c) => {
                    let id = c.id().clone();
                    self.idle_conns.lock().unwrap().push_back(c);
                    info!("Connection id:{}, Connecting to server {}:{}", id, host, port);
                },
                Err(e) => {
                    
                    error!("Connection id: Failed to create a connection to {}:{}. Error: {}",
                    host, port,e);
                    return false;
                }
            }
        }
        return true;
    }

    /// Release all :  Remove all connections  from th pool
    pub fn release_all(&self) {
        info!("release_all called");
        info!("It should trigger drop connection");
        self.idle_conns.lock().unwrap().clear();
        self.conns_inuse.store(0, Ordering::Relaxed);
        let total_count = self.idle_conns.lock().unwrap().len() +
                          self.conns_inuse.load(Ordering::Relaxed);
        info!("release_all called: Total_count: {}", total_count);
    }



    ///Releae connection
    #[allow(dead_code)]
    pub fn release(&self, conn: conn::Connection) {
        let a = self.idle_conns.lock();
        let conn_inuse = self.conns_inuse.load(Ordering::Relaxed);
        let id = conn.id().clone();
        let is_valid = conn.is_valid();
        
        let idle_count = a.unwrap().len();
        let total = idle_count  + conn_inuse;
        
        info!("release(): conn id:{}, min_conn:{}, idle connection: {}, connection in use:{},  total: {}", id, self.min_conns, idle_count, conn_inuse, total);
        
        if total < self.min_conns  && is_valid {
            info!("Pushing back to ideal_conns");
            self.idle_conns.lock().unwrap().push_back(conn);
            self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
            return;
        } 
        if !is_valid {
            self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
            info!("Connection not valid. It should trigger drop connection");
        } else {
            //drop(conn);
            self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
            info!("conn id:{}:It should trigger drop connection from inuse", id);
        }
        info!("release() end: Total_count: {}",
              self.idle_conns.lock().unwrap().len() + self.conns_inuse.load(Ordering::Relaxed));
              
        let _ = a;  
    }

    /// Drop connection.  Use only if disconect.
    #[allow(unused_variables)]
    pub fn drop(&self, conn: conn::Connection) {
        self.conns_inuse.fetch_sub(1, Ordering::Relaxed);
        warn!("drop() end: Total_count: {}",
              self.idle_conns.lock().unwrap().len() + self.conns_inuse.load(Ordering::Relaxed));

    }


    /// Aquire Connection
    pub fn acquire(&self) -> Result<conn::Connection> {

        let mut conns = self.idle_conns.lock().unwrap();
        {
            if !conns.is_empty() {
                let result = conns.pop_front();
                if result.is_some() {
                    let conn = result.unwrap();
                    // self.inuse_conns.push_back(conn);
                    self.conns_inuse.fetch_add(1, Ordering::Relaxed);
                    return Ok(conn);
                }
            }
            info!("Allocating new connection");
            let total_count = conns.len() + self.conns_inuse.load(Ordering::Relaxed);
            if total_count >= self.max_conns && self.tmp_conn_allowed == false {
                return Err(Error::new(ErrorKind::Other,
                                      // desc: "No connection available",
                                      "Max pool size has reached and temporary connections are \
                                       not allowed."
                                          .to_string()));

            }
        }
        info!("*****Init:Creating connection..");
        let conn = conn::Connection::connect(&self.config);
        match conn {
            Ok(c) => {
                info!("New connection id:{}", c.id().clone());
                self.conns_inuse.fetch_add(1, Ordering::Relaxed);
                return Ok(c);
            }
            Err(e) => {
                error!("Failed to create a connection : {}", e);
                return Err(e);
            }
        }

    }
}

#[cfg(test)]
pub mod tests {
    use std::io::prelude::*;
    use std::net::{TcpListener, TcpStream};
    // use std::default::Default;
    use std::sync::Arc;
    use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
    use std::thread;
    use net::config;
    use std::str;
    // use std::io::{Read, Write};
    // use std::old_io;
    // use std::test;
    extern crate env_logger;
    use std::thread::sleep;
    use std::time::Duration;



    #[cfg(test)]
    #[allow(unused_variables)]
    pub fn listen_ip4_localhost(port: u16, rx: Receiver<isize>) {
        let uri = format!("127.0.0.1:{}", port);
        let acceptor = TcpListener::bind(&*uri).unwrap();

        // acceptor.set_timeout(Some(1000));
        for stream in acceptor.incoming() {

            match stream {
                Err(e) => debug!("Accept err {}", e),
                Ok(stream) => {
                    info!("Got new connection on port {}", port);
                    thread::spawn(move || {
                        let result = handle_client(stream);
                        // info!("{}", handle_client(stream).unwrap());
                    });
                }
            }
            match rx.try_recv() {
                Ok(_) => {
                    info!("******Terminating thread with port {}.", port);
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    info!("******Terminating thread with port {}.", port);
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        drop(acceptor);
    }
    #[cfg(test)]
    #[allow(unused_variables)]
    fn handle_client(mut stream: TcpStream) -> () {

        let mut buf = [0];
        loop {
            let got = stream.read(&mut buf).unwrap();
            if got == 0 {
                warn!("handle_client: Received: 0");
                let result = stream.write("Fail to read\r\n".as_bytes());
                // Ok(result)
                // stream.flush().unwrap();
                // return result;
                break;
            } else {
                debug!("handle_client: Received: {}. Sending it back",
                      str::from_utf8(&buf).unwrap());
                let result = stream.write(&buf[0..got]);
                // return result;
                //    Ok(result)
                // break;
            }
        }
        // Ok(())
    }
    #[cfg(test)]
    fn next_test_port() -> u16 {
        use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
        static NEXT_OFFSET: AtomicUsize = ATOMIC_USIZE_INIT;
        const BASE_PORT: u16 = 9600;
        BASE_PORT + NEXT_OFFSET.fetch_add(1, Ordering::Relaxed) as u16
    }


    #[test]
    fn test_new() {
        info!("test_new started---------");

        let cfg: config::Config = Default::default();
        let pool = super::ConnectionPool::new(0, 5, false, &cfg);
        assert_eq!(pool.idle_conns_count(), 0);
        sleep(Duration::from_millis(1000));
        pool.release_all();
        assert_eq!(pool.idle_conns_count(), 0);
        info!("test_new ended---------");
    }



    ///test google
    #[test]
    fn test_google() {

        info!("test_lib started---------");
        let mut cfg: config::Config = Default::default();
        cfg.port = Some(80); //Some(old_io::test::next_test_port());
        cfg.server = Some("google.com".to_string()); //Some("127.0.0.1".to_string());

        let pool = super::ConnectionPool::new(2, 20, true, &cfg);
        assert_eq!(pool.init(), true);
        assert_eq!(pool.idle_conns_count(), 2);
        let mut conn = pool.acquire().unwrap();
        assert_eq!(conn.is_valid(), true);
        assert_eq!(pool.idle_conns_count(), 1);
        conn.writer.write("GET google.com\r\n".as_bytes()).unwrap();
        conn.writer.flush().unwrap();
        let mut buffer = String::new();
        let r = conn.reader.read_line(&mut buffer);
        if r.unwrap() > 0 {
            println!("Received {}", buffer);
        }
        pool.release(conn);
        assert_eq!(pool.idle_conns_count(), 1);
        info!("test_lib ended---------");

    }

    #[test]
    fn test_init() {

        info!("test_init started---------");

        let mut cfg: config::Config = Default::default();
        cfg.port = Some(next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();



        {
            info!("test_init starting channel---------");
            let (tx, rx): (Sender<isize>, Receiver<isize>) = channel();
            info!("test_init spawning thread---------");
            thread::spawn(move || {
                info!("test_init calling listen_Ip4_localhost with port {}",
                      listen_port);
                listen_ip4_localhost(listen_port, rx);
            });
            sleep(Duration::from_millis(500));
            info!("test_init starting connection pool");
            let pool = super::ConnectionPool::new(1, 5, false, &cfg);
            assert_eq!(pool.init(), true);
            assert_eq!(pool.idle_conns_count(), 1);
            let mut c1 = pool.acquire().unwrap();
            info!("test_init acquire connection");
            assert_eq!(c1.is_valid(), true);
            info!("test_init send data");
            c1.writer.write("GET google.com\r\n".as_bytes()).unwrap();
            c1.writer.flush().unwrap();
            info!("reading line");
            let mut buffer = String::new();
            let r = c1.reader.read_line(&mut buffer);
            if r.unwrap() > 0 {
                println!("Received {}", buffer);
            }
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

        info!("test_example started---------");
     //   env_logger::init().unwrap();
        let mut cfg: config::Config = Default::default();
        cfg.port = Some(next_test_port());
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<isize>, Receiver<isize>) = channel();
        thread::spawn(move || {
            listen_ip4_localhost(listen_port, rx);
        });
        
        let pool = super::ConnectionPool::new(2, 5, true, &cfg);
        let pool_shared = Arc::new(pool);
        let mut ts = Vec::new();
       
        for _ in 0u32..6 {
            let pool = pool_shared.clone();
            let t = thread::spawn(move || {
                warn!("test_example error---------");
                let mut conn =   pool.acquire().unwrap(); 
                warn!("test_example error---------");
                conn.writer.write("GET google.com\r\n".as_bytes());
                conn.writer.flush();
                let mut buffer = String::new();
                let r = conn.reader.read_line(&mut buffer);
                match r {
                    Ok(v) => {
                     if v > 0 {
                        println!("Received {}", buffer);
                     }
                    },
                    Err(e) => println!("error : {:?}", e),

                }
                pool.release(conn);
            });
            ts.push(t);
        }
        let l = ts.len();
        for _ in 0..l {
            let t = ts.pop();
            t.unwrap().join();
        }
        sleep(Duration::from_millis(500));
        assert_eq!(pool_shared.idle_conns_count(), 1);
        pool_shared.release_all();
        assert_eq!(pool_shared.idle_conns_count(), 0);
        tx.send(0);
        info!("test_example ended---------");
    }

    #[test]
    fn test_acquire_release_1() {
        let _ = env_logger::init();
        // log::set_logger(Box::new( custlogger::CustLogger { handle: stderr() }) );
        info!("test_acquire_release started---------");
        let mut cfg: config::Config = Default::default();

        cfg.port = Some(next_test_port() );
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<isize>, Receiver<isize>) = channel();
        thread::spawn(move || {
            listen_ip4_localhost(listen_port, rx);
        });
        sleep(Duration::from_millis(1000));
        {
            let pool = super::ConnectionPool::new(2, 2, true, &cfg);
            assert_eq!(pool.init(), true);
            assert_eq!(pool.idle_conns_count(), 2);

            let c1 = pool.acquire().unwrap();
            info!("c1: {}", pool.idle_conns_count());
            assert_eq!(pool.idle_conns_count(), 1);
            let c2 = pool.acquire().unwrap();
            info!("c2: {}", pool.idle_conns_count());
            assert_eq!(pool.idle_conns_count(), 0);
            let c3 = pool.acquire().unwrap();
            pool.release(c1);
            assert_eq!(pool.idle_conns_count(), 0);
            pool.release(c2);
            assert_eq!(pool.idle_conns_count(), 0);
            pool.release(c3);
            assert_eq!(pool.idle_conns_count(), 1);

            pool.release_all();
            assert_eq!(pool.idle_conns_count(), 0);
            tx.send(0);
        }

        info!("test_acquire_release ended---------");
    }

    #[test]
    fn test_acquire_release_multithread() {
        //sleep(Duration::from_millis(2000));
        info!("test_acquire_release_multithread started---------");
        let mut cfg: config::Config = Default::default();
        cfg.port = Some(next_test_port() + 10);
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<isize>, Receiver<isize>) = channel();
        {
            thread::spawn(move || {
                listen_ip4_localhost(listen_port, rx);
            });

            sleep(Duration::from_millis(1000));

            let pool = super::ConnectionPool::new(2, 10, true, &cfg);
            assert_eq!(pool.init(), true);
            let pool_shared = Arc::new(pool);
            for _ in 0u32..10 {
                let p1 = pool_shared.clone();
                thread::spawn(move || {
                    let c1 = p1.acquire().unwrap();
                    let c2 = p1.acquire().unwrap();
                    let c3 = p1.acquire().unwrap();
                    p1.release(c1);
                    p1.release(c2);
                    p1.release(c3);
                });
            }
            sleep(Duration::from_millis(2000));
            assert_eq!(pool_shared.idle_conns_count(), 1);
            pool_shared.release_all();
            assert_eq!(pool_shared.idle_conns_count(), 0);
            tx.send(0);
        }

        // assert_eq!(pool_shared.idle_conns_count(), 10);
        info!("test_acquire_release_multithread ended---------");
    }

    #[test]
    fn test_acquire_release_multithread_2() {

        info!("test_acquire_release_multithread_2 started---------");
        let mut cfg: config::Config = Default::default();
        cfg.port = Some(next_test_port() );
        cfg.server = Some("127.0.0.1".to_string());
        let listen_port = cfg.port.unwrap();
        let (tx, rx): (Sender<isize>, Receiver<isize>) = channel();
        thread::spawn(move || {
            listen_ip4_localhost(listen_port, rx);
        });
        sleep(Duration::from_millis(1000));
        let pool = super::ConnectionPool::new(2, 3, true, &cfg);
        assert_eq!(pool.init(), true);
        let pool_shared = Arc::new(pool);
        for _ in 0u32..2 {
            let p1 = pool_shared.clone();
            thread::spawn(move || {
                info!("test_acquire_release_multithread_2 acquired connection in thread");
                let c1 = p1.acquire().unwrap();
                info!("test_acquire_release_multithread_2 release connection in thread");
                p1.release(c1);

            });
        }
        sleep(Duration::from_millis(500));
        info!("test_acquire_release_multithread_2 out of for loop :{}",
              pool_shared.idle_conns_count());
        assert_eq!(pool_shared.idle_conns_count(), 1);
        pool_shared.release_all();
        assert_eq!(pool_shared.idle_conns_count(), 0);
        tx.send(0);

        info!("test_acquire_release_multithread_2 ended---------");
    }

    #[test]
    #[cfg(feature = "ssl")]
    fn test_init_ssl() {
        info!("test_init_ssl started---------");
        let mut cfg: config::Config = Default::default();
        cfg.port = Some(443);
        cfg.server = Some("google.com".to_string());

        cfg.use_ssl = Some(true);
        let pool = super::ConnectionPool::new(2, 5, false, &cfg);
        assert_eq!(pool.init(), true);

        pool.release_all();
        assert_eq!(pool.idle_conns_count(), 0);
        info!("test_init_ssl ended---------");
    }
}
