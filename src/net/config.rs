//! Configuration for connection object
//! #![unstable]
use std::default::Default;
use std::path::PathBuf;
use std::time::Duration;

///Configuration data.
#[derive(Clone)]
pub struct Config {
    /// The server to connect to.
    pub server: Option<String>,
    /// The port to connect to.
    pub port: Option<u16>,
    /// Read timeout.
    pub read_timeout: Option<Duration>,
    /// Write timeout.
    pub write_timeout: Option<Duration>,
    ///If true, it will assume ssl is enabled
    pub use_ssl: Option<bool>,
    /// SSL Protocol
    //pub ssl_protocol : Option<>,
    /// Certificate File
    pub certificate_file: Option<PathBuf>,
    /// Private Key File
    pub private_key_file: Option<PathBuf>,
    /// CA File
    pub ca_file: Option<PathBuf>,
    /// Verify certificate
    pub verify: Option<bool>,
    /// Verify depth
    pub verify_depth: Option<u32>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            server: Some("localhost".to_string()),
            port: Some(21950),
            read_timeout: Some(Duration::from_millis(60_000)),
            write_timeout: Some(Duration::from_millis(60_000)),
            use_ssl: Some(false),
           // ssl_protocol: 
            certificate_file: None,
            private_key_file: None,
            ca_file: None,
            verify: None,
            verify_depth: None,
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::default::Default;
    use std::time::Duration;
    #[test]
    fn test_config() {
        let c = super::Config {
            server: Some("localhost".to_string()),
            port: Some(2195),
            ..Default::default()
        };
        assert_eq!(c.port, Some(2195));
        assert_eq!(c.read_timeout, Some(Duration::from_millis(60_000)));
    }
}
