use log::{Logger,LogRecord, set_logger};
use std::io::{IoResult,  IoError, IoErrorKind, LineBufferedWriter, stdio, stderr} ;

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

/// usage: log::set_logger(Box::new( CustLogger { handle: stderr() }) );
