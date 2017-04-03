//! Client Connection.  It supports unsecured and secured(SSL) connection
//#[cfg(feature = "ssl")]
//use std::borrow::ToOwned;
#[cfg(feature = "ssl")]
use std::error::Error as StdError;
#[cfg(feature = "ssl")]
use std::io::{ErrorKind, Error};
#[cfg(feature = "ssl")]
use std::result::Result as StdResult;
use std::io::{Write, Read, Result, BufReader, BufWriter};
use std::net::TcpStream;
#[cfg(test)]
use std::net::Shutdown;
use std::os::unix::prelude::AsRawFd;

#[cfg(feature = "ssl")]
use openssl::ssl::{SslContext, Ssl, SslMethod, SslStream, SSL_VERIFY_PEER};
#[cfg(feature = "ssl")]
use openssl::error::ErrorStack;
#[cfg(feature = "ssl")]
use openssl::x509;

// use std::bool;
use net::config;
use uuid::Uuid;

// pub mod config;

/// A Connection object.  Make sure you syncronize if uses in multiple threads
pub struct Connection {
    id: String,
    /// BufReader for NetStream (TCP/SSL)
    pub reader: BufReader<NetStream>,
    /// BufWriter for NetStream (TCP/SSL)
    pub writer: BufWriter<NetStream>,
    /// Config for connection
    config: config::Config,
}

/// Implementation for Connectio
impl Connection {
    /// new function to create default Connection object
    fn new(reader: BufReader<NetStream>,
           writer: BufWriter<NetStream>,
           config: &config::Config)
           -> Connection {
        Connection {
            id: Uuid::new_v4().to_urn_string(),
            reader: reader,
            writer: writer,
            config: config.clone()
        }
    }

    /// connection unique id
    
    /// Creates a  TCP connection to the specified server.

    pub fn connect(config: &config::Config) -> Result<Connection> {
        if config.use_ssl.unwrap_or(false) {
            Connection::connect_ssl_internal(config)
        } else {
            Connection::connect_internal(config)
        }
    }

    /// Creates a  TCP/SSL connection to the specified server.
    ///If already connected, it will drop and reconnect

    pub fn reconnect(&mut self) -> Result<Connection> {
        if self.config.use_ssl.unwrap_or(false) {
            Connection::connect_ssl_internal(&self.config)
        } else {
            Connection::connect_internal(&self.config)
        }
    }
    
    /// Get the connection id
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Is Valid connection
    pub fn is_valid(&self) -> bool {
        match self.reader.get_ref() {
            &NetStream::UnsecuredTcpStream(ref tcp) => {
                debug!("TCP FD:{}", tcp.as_raw_fd());
                if tcp.as_raw_fd() < 0 {
                     false
                } else {
                     true
                }
            }
            #[cfg(feature = "ssl")]
            &NetStream::SslTcpStream(ref ssl) => {
                debug!("SSL FD:{}", ssl.get_ref().as_raw_fd());
                if ssl.get_ref().as_raw_fd() < 0 {
                    return false;
                } else {
                    return true;
                }
            }
        }
    }


    /// Creates a TCP connection with an optional timeout.

    fn connect_internal(config: &config::Config) -> Result<Connection> {
        let host: &str = &config.server.clone().unwrap();
        let port = config.port.unwrap();
        info!("Connecting to server {}:{}", host, port);
        let reader_socket = try!(TcpStream::connect((host, port)));
        let writer_socket = try!(reader_socket.try_clone());
        // fixme:  socket.set_timeout(config.connect_timeout);
        Ok(Connection::new(BufReader::new(NetStream::UnsecuredTcpStream(reader_socket)),
                           BufWriter::new(NetStream::UnsecuredTcpStream(writer_socket)),
                           config))
    }



    /// Panics because SSL support was not included at compilation.
    #[cfg(not(feature = "ssl"))]
    fn connect_ssl_internal(config: &config::Config) -> Result<Connection> {
        panic!("Cannot connect to {}:{} over SSL without compiling with SSL support.",
               config.server.clone().unwrap(),
               config.port.unwrap())
    }

    /// Creates a  TCP connection over SSL.
    #[cfg(feature = "ssl")]
    fn connect_ssl_internal(config: &config::Config) -> Result<Connection> {
        let host: &str = &config.server.clone().unwrap();
        let port = config.port.unwrap();
        info!("Connecting to server {}:{}", host, port);
        // let mut socket = try!(TcpStream::connect(format!("{}:{}", server, port)[]));
        let reader_socket = try!(TcpStream::connect((host, port)));
         let writer_socket = try!(reader_socket.try_clone());
        //reader_socket.set_timeout(config.connect_timeout);
        // writer_socket.set_timeout(config.connect_timeout);
        
        let mut ctx = try!(ssl_to_io(SslContext::builder(SslMethod::tls())));
        

        // verify peer
        if config.verify.unwrap_or(false) {
            ctx.set_verify(SSL_VERIFY_PEER);
        }

        // verify depth
        if config.verify_depth.unwrap_or(0) > 0 {
            ctx.set_verify_depth(config.verify_depth.unwrap());
        }
        if config.certificate_file.is_some() {
        try!(ssl_to_io(ctx.set_certificate_file(config.certificate_file.as_ref().unwrap(),
                                           x509::X509_FILETYPE_PEM)));
        }
        if config.private_key_file.is_some() {
            try!(ssl_to_io(ctx.set_private_key_file(config.private_key_file.as_ref().unwrap(),
                                                           x509::X509_FILETYPE_PEM)));
        }
        if config.ca_file.is_some() {
            try!(ssl_to_io(ctx.set_ca_file(config.ca_file.as_ref().unwrap())));
        }
        let ctx_x = ctx.build();
        error!("HERE");
        let ssl_read = Ssl::new(&ctx_x).unwrap();
        let ssl_write = Ssl::new(&ctx_x).unwrap();
        //let read_socket = try!(socket.try_clone());
        
        error!("HERE1");
        let ssl_read_socket =  ssl_read.connect(reader_socket).unwrap();
        error!("HERE2");
       
       let ssl_write_socket = ssl_write.connect(writer_socket).unwrap();
        error!("HERE3");
        Ok(Connection::new(BufReader::new(NetStream::SslTcpStream(ssl_read_socket)),
                           BufWriter::new(NetStream::SslTcpStream(ssl_write_socket)),
                           config))

    }
}


/// Converts a Result<T, SslError> isizeo an Result<T>.
#[cfg(feature = "ssl")]
fn ssl_to_io<T>(res: StdResult<T, ErrorStack>) -> Result<T> {
    match res {
        Ok(x) => Ok(x),
        Err(e) => {
            Err(Error::new(ErrorKind::Other,
                           &format!("An SSL error occurred. ({})", e.description())[..]))
        }
    }
}


/// An abstraction over different networked streams.

pub enum NetStream {
    /// An unsecured TcpStream.
    UnsecuredTcpStream(TcpStream),
    /// An SSL-secured TcpStream.
    /// This is only available when compiled with SSL support.
    #[cfg(feature = "ssl")]
    SslTcpStream(SslStream<TcpStream>),
}
// trait Reader {
//     fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
// }
impl Read for NetStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            &mut NetStream::UnsecuredTcpStream(ref mut stream) => stream.read(buf),
            #[cfg(feature = "ssl")]
            &mut NetStream::SslTcpStream(ref mut stream) => stream.read(buf),
        }
    }
}
// trait Writer {
//     fn write(&mut self, buf: &[u8]) -> Result<()>;
//     fn write_all(&mut self, buf: &[u8]) -> Result<()>;
// }
impl Write for NetStream {
    fn write(&mut self, buf: &[u8]) -> Result<(usize)> {
        match self {
            &mut NetStream::UnsecuredTcpStream(ref mut stream) => stream.write(buf),
            #[cfg(feature = "ssl")]
            &mut NetStream::SslTcpStream(ref mut stream) => stream.write(buf),
        }
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        match self {
            &mut NetStream::UnsecuredTcpStream(ref mut stream) => stream.write_all(buf),
            #[cfg(feature = "ssl")]
            &mut NetStream::SslTcpStream(ref mut stream) => stream.write_all(buf),
        }
    }
    fn flush(&mut self) -> Result<()> {
        match self {
            &mut NetStream::UnsecuredTcpStream(ref mut stream) => stream.flush(),
            #[cfg(feature = "ssl")]
            &mut NetStream::SslTcpStream(ref mut stream) => stream.flush(),
        }
    }
}


#[cfg(test)]
#[allow(unused_must_use)]
impl Drop for Connection {
    ///drop method
    fn drop(&mut self) {
        info!("Drop for Connection:Dropping connection id: {}", self.id.clone());
        match self.reader.get_mut() {
            &mut NetStream::UnsecuredTcpStream(ref mut stream) => {
                stream.shutdown(Shutdown::Read);
                stream.shutdown(Shutdown::Write);
            }
            #[cfg(feature = "ssl")]
            &mut NetStream::SslTcpStream(ref mut ssl) => {
                ssl.get_mut().shutdown(Shutdown::Read);
                ssl.get_mut().shutdown(Shutdown::Write);
            }
        }
    }
}
