//! Client Connection.  It supports unsecured and secured(SSL) connection
#![experimental]
use std::io::{BufferedReader, BufferedWriter, IoResult,  IoError, IoErrorKind, TcpStream};
use std::os::unix::prelude::AsRawFd;

#[cfg(feature = "ssl")] use openssl::ssl::{SslContext, SslMethod, SslStream, SslVerifyMode};
#[cfg(feature = "ssl")] use openssl::ssl::error::SslError;
#[cfg(feature = "ssl")] use openssl::x509;
//use std::bool;
use net::config;
//pub mod config;


/// A Connection object.  Make sure you syncronize if uses in multiple threads
#[experimental]
pub struct Connection  {
    /// BufferedReader for NetStream (TCP/SSL)
    pub reader: BufferedReader<NetStream>,
    /// BufferedWriter for NetStream (TCP/SSL)
    pub writer: BufferedWriter<NetStream>,
    /// Config for connection
    config: config::Config,
}

/// Implementation for Connectio
impl  Connection {
     /// new function to create default Connection object
     fn new (reader: BufferedReader<NetStream>, writer: BufferedWriter<NetStream>, config: &config::Config) -> Connection {
        Connection {
            reader: reader,
            writer: writer,
            config: config.clone(),
        }
     }
    /// Creates a  TCP connection to the specified server.
    #[experimental]
    pub fn connect(config: &config::Config) -> IoResult<Connection> {
        if config.use_ssl.unwrap_or(false)  {
           Connection::connect_ssl_internal(config)
        }else {
           Connection::connect_internal(config)
        }
    }

    /// Creates a  TCP/SSL connection to the specified server. If already connected, it will drop and reconnect
    #[experimental]
    pub fn reconnect(&mut self) ->  IoResult<Connection> {
        if self.config.use_ssl.unwrap_or(false)  {
            Connection::connect_ssl_internal(&self.config)
        }else {
            Connection::connect_internal(&self.config)
        }
    }

    /// Is Valid connection
    pub fn is_valid(&self) -> bool {
        match self.reader.get_ref() {
             &NetStream::UnsecuredTcpStream(ref tcp)  => { 
                debug!("TCP FD:{}", tcp.as_raw_fd());
                if tcp.as_raw_fd() < 0  { 
                    
                    return false; 
                } else { 
                    return true; 
                } 
            },  
             #[cfg(feature = "ssl")]
            &NetStream::SslTcpStream (ref ssl) =>  {
              debug!("SSL FD:{}", ssl.get_ref().as_raw_fd());
              if ssl.get_ref().as_raw_fd() < 0 {
                  return false; 
              } else { 
                return true; 
              } 
            },       
        }
    }

    
    /// Creates a TCP connection with an optional timeout.
    #[experimental]
    fn connect_internal(config: &config::Config) -> IoResult<Connection> {  
        info!("Connecting to server {}:{}", config.server.clone().unwrap(), config.port.unwrap());
        let mut socket = try!(TcpStream::connect(format!("{}:{}", config.server.clone().unwrap(), config.port.unwrap())[]));
        socket.set_timeout(config.connect_timeout);
        Ok(Connection::new(
            BufferedReader::new(NetStream::UnsecuredTcpStream(socket.clone())),
            BufferedWriter::new(NetStream::UnsecuredTcpStream(socket)),
            config,
        ))
    }
    

    
    /// Panics because SSL support was not included at compilation.
    #[experimental]
    #[cfg(not(feature = "ssl"))]
    fn connect_ssl_internal(config: &config::Config) -> IoResult<Connection> {
        panic!("Cannot connect to {}:{} over SSL without compiling with SSL support.", config.server, config.port)
    }

    /// Creates a  TCP connection over SSL.
    #[experimental]
    #[cfg(feature = "ssl")]
    fn connect_ssl_internal(config: &config::Config) -> IoResult<Connection> {
          info!("Connecting to server {}:{}", config.server.clone().unwrap(), config.port.unwrap());
      //  let mut socket = try!(TcpStream::connect(format!("{}:{}", server, port)[]));
       let mut socket = try!(TcpStream::connect(format!("{}:{}", config.server.clone().unwrap(), config.port.unwrap())[]));
        socket.set_timeout(config.connect_timeout);

        let mut ssl = try!(ssl_to_io(SslContext::new(SslMethod::Tlsv1)));

        //set ssl options
        try!(Connection::set_ssl_options(&mut ssl, config));
       
        let ssl_socket = try!(ssl_to_io(SslStream::new(&ssl, socket)));
        Ok(Connection::new(
                BufferedReader::new(NetStream::SslTcpStream(ssl_socket.clone())),
                BufferedWriter::new(NetStream::SslTcpStream(ssl_socket)),
                config,
        ))

    }
    
    /// Set the SSL certs and verification options
     #[experimental]
    #[cfg(feature = "ssl")]
    fn set_ssl_options(ssl: &mut SslContext, config: &config::Config) -> Result<(),IoError> {
         //verify peer
        if config.verify.unwrap_or(false) {
            ssl.set_verify(SslVerifyMode::SslVerifyPeer, None);
        }

        //verify depth
        if config.verify_depth.unwrap_or(0) > 0 {
            ssl.set_verify_depth(config.verify_depth.unwrap());
        }  

         if config.certificate_file.is_some() {
            try!(ssl_option_to_io(ssl.set_certificate_file(config.certificate_file.as_ref().unwrap(), x509::X509FileType::PEM)));
        }
        //load private key if populated
        if config.private_key_file.is_some() {
            try!(ssl_option_to_io(ssl.set_private_key_file(config.private_key_file.as_ref().unwrap(), x509::X509FileType::PEM)));
        }
        
        //load cafile if populated
       if config.ca_file.is_some() {
            try!(ssl_option_to_io(ssl.set_CA_file(config.ca_file.as_ref().unwrap())));
        }
        Ok(())
    }


}


/// Converts a Result<T, SslError> into an IoResult<T>.
#[cfg(feature = "ssl")]
fn ssl_to_io<T>(res: Result<T, SslError>) -> IoResult<T> {
    match res {
        Ok(x) => Ok(x),
        Err(e) => Err(IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "An SSL error occurred.",
            detail: Some(format!("{}", e)),
        }),
    }
}

/// Converts a Result<T, SslError> into an IoResult<T>.
#[cfg(feature = "ssl")]
fn ssl_option_to_io(res: Option<SslError>) -> Result<(),IoError> {
    match res {
        None => Ok(()),
        Some(e) => Err(IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "An SSL error occurred.",
            detail: Some(format!("{}", e)),
        }),
    }
}


/// An abstraction over different networked streams.
#[experimental]
pub enum NetStream {
    /// An unsecured TcpStream.
    UnsecuredTcpStream(TcpStream),
    /// An SSL-secured TcpStream.
    /// This is only available when compiled with SSL support.
    #[cfg(feature = "ssl")]
    SslTcpStream(SslStream<TcpStream>),
}

impl Reader for NetStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        match self {
            &NetStream::UnsecuredTcpStream(ref mut stream) => stream.read(buf),
            #[cfg(feature = "ssl")]
            &NetStream::SslTcpStream(ref mut stream) => stream.read(buf),
        }
    }
}

impl Writer for NetStream {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        match self {
            &NetStream::UnsecuredTcpStream(ref mut stream) => stream.write(buf),
            #[cfg(feature = "ssl")]
            &NetStream::SslTcpStream(ref mut stream) => stream.write(buf),
        }
    }
}


#[cfg(test)]
#[allow(unused_must_use)]
impl Drop for Connection {
    ///drop method 
    fn drop(&mut self) {
        info!("Drop for Connection:Dropping connection!");
        match self.reader.get_mut() {
             &NetStream::UnsecuredTcpStream(ref mut stream) => {
                  stream.close_read();
                 stream.close_write();
            },  
             #[cfg(feature = "ssl")]
            &NetStream::SslTcpStream (ref mut ssl) =>  {           
               ssl.get_mut().close_read();
              ssl.get_mut().close_write();
            },       
        }
    }
}



