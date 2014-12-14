//! Client Connection.  It supports unsecured and secured(SSL) connection
#![experimental]
use std::io::{BufferedReader, BufferedWriter, IoResult, InvalidInput, IoError, IoErrorKind, TcpStream};
use std::sync::{ Arc, Mutex };

#[cfg(feature = "ssl")] use openssl::ssl::{SslContext, SslMethod, SslStream, SslVerifyMode};
#[cfg(feature = "ssl")] use openssl::ssl::error::SslError;
#[cfg(feature = "ssl")] use openssl::x509;
//use std::bool;
use net::config;
//pub mod config;


/// A Connection object.  Make sure you syncronize if uses in multiple threads
#[experimental]
pub struct Connection  {
    pub reader: BufferedReader<NetStream>,
    pub writer: BufferedWriter<NetStream>,
    pub config: config::Config,
}

impl  Connection {
     
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

         //verify peer
        if config.verify.unwrap_or(false) {
            ssl.set_verify(SslVerifyMode::SslVerifyPeer, None);
        }

        //verify depth
        if config.verify_depth.unwrap_or(0) > 0 {
            ssl.set_verify_depth(config.verify_depth.unwrap());
        }

        let mut r = None;      

         // load cert if populated
        if config.certificate_file.is_some() {
             //let cf = Arc::new(config.certificate_file.unwrap());
            r = ssl.set_certificate_file(config.certificate_file.as_ref().unwrap(), x509::X509FileType::PEM);
        }
        //load private key if populated
        if config.private_key_file.is_some() {
            r = ssl.set_private_key_file(config.private_key_file.as_ref().unwrap(), x509::X509FileType::PEM);
        }
        //load cafile if populated
        if config.ca_file.is_some() {
            r = ssl.set_CA_file(config.ca_file.as_ref().unwrap());
        }
        
      //  let r = Connection::set_ssl_options(&mut ssl, config);
        match r {
            None => debug!("Success"),
            Some(e) => { return Err(IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "An SSL error occurred.",
                detail: Some(format!("{}", e)),
            }); },
        }
       
        let ssl_socket_result = ssl_to_io(SslStream::new(&ssl, socket));

        match ssl_socket_result {
            Ok(ssl_socket) => Ok(Connection::new(
                BufferedReader::new(NetStream::SslTcpStream(ssl_socket.clone())),
                BufferedWriter::new(NetStream::SslTcpStream(ssl_socket)),
                config,
            )),
            Err(e) => Err(e)
        }
    }
    /*
    /// Set the SSL certs and verification options
    fn set_ssl_options(ssl: &mut SslContext, config: &config::Config) -> Option<IoError> {

        let r = None;
       
         // load cert if populated
        if config.certificate_file.is_some() {
            r = ssl_option_to_io(ssl.set_certificate_file(&config.certificate_file.unwrap(), x509::X509FileType::PEM));
        }
        //load private key if populated
        if config.private_key_file.is_some() {
            r = ssl_option_to_io(ssl.set_private_key_file(&config.private_key_file.unwrap(), x509::X509FileType::PEM));
        }
        //load cafile if populated
        if config.ca_file.is_some() {
            r = ssl_option_to_io(ssl.set_CA_file(&config.ca_file.unwrap()));
        }
        return r;
        
    }*/


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
fn ssl_option_to_io(res: Option<SslError>) -> Option<IoError> {
    match res {
        None => None,
        Some(e) => Some(IoError {
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


//#[cfg(test)]
impl Drop for Connection {
    ///drop method 
    fn drop(&mut self) {
        warn!("Dropping connection!");
    }
}



