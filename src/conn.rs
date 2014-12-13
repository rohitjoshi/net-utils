//! Client Connection.  It supports unsecured and secured(SSL) connection
#![experimental]
use std::io::{BufferedReader, BufferedWriter, IoResult, InvalidInput, IoError, IoErrorKind, TcpStream};


#[cfg(feature = "ssl")] use openssl::ssl::{SslContext, SslMethod, SslStream};
#[cfg(feature = "ssl")] use openssl::ssl::error::SslError;
//use std::bool;
use config;
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

        let mut socket = try!(TcpStream::connect(format!("{}:{}", host, port)[]));
        socket.set_timeout(timeout_ms);

        let ssl = try!(ssl_to_io(SslContext::new(SslMethod::Tlsv1)));
        // load cert if populated
        if config.certificate_file.is_some() {
            try!(ssl_to_io(ssl.set_certificate_file(Some(config.certificate_file), X509FileType::PEM)));
        }
        //load private key if populated
        if config.private_key_file.is_some() {
            try!(ssl_to_io(ssl.set_private_key_file(Some(config.private_key_file), X509FileType::PEM)));
        }
        //load cafile if populated
        if config.ca_file.is_some() {
            try!(ssl_to_io(ssl.set_CA_File(Some(config.ca_file))));
        }

        //verify peer
        if config.verify.unwrap_or(false) {
            try!(ssl_to_io(ssl.set_verify(SslVerifyMode::SslVerifyPeer)));
        }

        //verify depth
        if config.verify_depth.unwrap_or(false) {
            try!(ssl_to_io(ssl.set_verify_depth(Some(config.verify_depth))));
        }

        let ssl_socket = try!(ssl_to_io(SslStream::new(&ssl, socket)));
        Ok(Connection::new(
            BufferedReader::new(NetStream::SslTcpStream(ssl_socket.clone())),
            BufferedWriter::new(NetStream::SslTcpStream(ssl_socket)),
        ));
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



