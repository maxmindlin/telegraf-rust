//! Telegraf-rust provides a lightweight client library for writing metrics
//! to a InfluxDB Telegraf service.
//!
//! This library does not provide querying or other InfluxDB client-library
//! features. This is meant to be lightweight and simple for services
//! to report metrics.
//!
//! # How to use
//!
//! All usage will start by creating a socket connection via a [crate::Client]. This
//! supports multiple connection protocols - which one you use will be determined
//! by how your Telegraf `input.socket_listener` configuration is setup.
//!
//! Once a client is setup there are multiple different ways to write points.
//!
//! ## Define structs that represent metrics using the derive macro.
//!
//! ```no_run
//! use telegraf::*;
//!
//! let mut client = Client::new("tcp://localhost:8094").unwrap();
//!
//! #[derive(Metric)]
//! struct MyMetric {
//!     field1: i32,
//!     #[telegraf(tag)]
//!     tag1: String,
//! }
//!
//! let point = MyMetric { field1: 1, tag1: "tag".to_owned() };
//! client.write(&point);
//! ```
//!
//! By default the measurement name will be the same as the struct. You can
//! override this via derive attributes:
//!
//! ```
//! use telegraf::*;
//!
//! #[derive(Metric)]
//! #[measurement = "custom_name"]
//! struct MyMetric {
//!     field1: i32,
//! }
//! ```
//!
//! As with any Telegraf point, tags are optional but at least one field
//! is required.
//!
//! ## Use the [crate::point] macro to do ad-hoc metrics.
//!
//! ```no_run
//! use telegraf::*;
//!
//! let mut client = Client::new("tcp://localhost:8094").unwrap();
//!
//! let p = point!("measurement", ("tag1", "tag1Val"), ("field1", "field1Val"));
//! client.write_point(&p);
//! ```
//!
//! The macro syntax is the following format:
//!
//! `(<measurement>, [(<tagName>, <tagVal>)], [(<fieldName>, <fieldVal>)])`
//!
//! Measurement name, tag set, and field set are space separated. Tag and field sets are space
//! separated. The tag set is optional.
//!
//!  Manual [crate::Point] initialization.
//!
//! ```no_run
//! use telegraf::{Client, Point};
//!
//! let mut c = Client::new("tcp://localhost:8094").unwrap();
//!
//! let p = Point::new(
//!     String::from("measurement"),
//!     vec![
//!         (String::from("tag1"), String::from("tag1value"))
//!     ],
//!     vec![
//!         (String::from("field1"), Box::new(10)),
//!         (String::from("field2"), Box::new(20.5)),
//!         (String::from("field3"), Box::new("anything!"))
//!     ]
//! );
//!
//! c.write_point(&p);
//! ```
//!
//! ### Field Data
//!
//! Any attribute that will be the value of a field must implement the `IntoFieldData` trait provided by this library.
//!
//! ```
//! use telegraf::FieldData;
//!
//! pub trait IntoFieldData {
//!     fn into_field_data(&self) -> FieldData;
//! }
//! ```
//!
//! Out of the box implementations are provided for many common data types, but manual implementation is possible for other data types.

pub mod macros;
pub mod protocol;

use std::fmt;
use std::io;
use std::io::{Write, Error};
use std::net::SocketAddr;
use std::net::UdpSocket;
use url::Url;
use std::net::{Shutdown, TcpStream};

use protocol::*;
pub use protocol::{IntoFieldData, FieldData};
pub use telegraf_derive::*;

/// Common result type. Only meaningful response is
/// an error.
pub type TelegrafResult = Result<(), TelegrafError>;

/// Trait for writing custom types as a telegraf
/// [crate::Point].
///
/// For most use cases it is recommended to
/// derive this trait instead of manually
/// implementing it.
///
/// Used via [crate::Client::write].
///
/// # Examples
///
/// ```
/// use telegraf::*;
///
/// #[derive(Metric)]
/// #[measurement = "my_metric"]
/// struct MyMetric {
///     field1: i32,
///     #[telegraf(tag)]
///     tag1: String,
///     field2: f32,
/// }
/// ```
pub trait Metric {
    /// Converts internal attributes
    /// to a Point format.
    fn to_point(&self) -> Point;
}

/// Error enum for library failures.
#[derive(Debug)]
pub enum TelegrafError {
    /// Error reading or writing I/O.
    IoError(Error),
    /// Error with internal socket connection.
    ConnectionError(String),
    /// Error when a bad protocol is created.
    BadProtocol(String)
}

/// A single influx metric. Handles conversion from Rust types
/// to influx lineprotocol syntax.
///
/// Telegraf protocol requires at least one field, whereas
/// tags are completely optional. Attempting to write a point
/// without any fields will return a [crate::TelegrafError].
///
/// Creation of points is made easier via the [crate::point] macro.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub measurement: String,
    pub tags: Vec<Tag>,
    pub fields: Vec<Field>
}

/// Connection client used to handle socket connection management
/// and writing.
pub struct Client {
    conn: Connector
}

/// Different types of connections that the library supports.
enum Connector {
    TCP(TcPConnection),
    UDP(UdPConnection),
}

/// TCP socket connection container.
struct TcPConnection {
    conn: TcpStream
}

struct UdPConnection {
    conn: UdpSocket
}

impl Point {
    /// Creates a new Point that can be written using a [Client].
    pub fn new(
        measurement: String,
        tags: Vec<(String, String)>,
        fields: Vec<(String, Box<dyn IntoFieldData>)>,
    ) -> Self {
        let t = tags.into_iter()
            .map(|(n,v)| Tag { name: n, value: v })
            .collect();
        let f = fields.into_iter()
            .map(|(n,v)| Field { name: n, value: v.into_field_data() })
            .collect();
        Self {
            measurement,
            tags: t,
            fields: f,
        }
    }

    fn to_lp(&self) -> LineProtocol {
        let tag_attrs: Vec<Attr> = self.tags
            .to_owned()
            .into_iter()
            .map(Attr::Tag)
            .collect();
        let field_attrs: Vec<Attr> = self.fields
            .to_owned()
            .into_iter()
            .map(Attr::Field)
            .collect();
        let tag_str = format_attr(tag_attrs);
        let field_str = format_attr(field_attrs);
        LineProtocol::new(self.measurement.clone(), tag_str, field_str)
    }
}

impl Client {
    /// Creates a new Client. Determines socket protocol from
    /// provided URL.
    pub fn new(conn_url: &str) -> Result<Self, TelegrafError> {
        let conn = Connector::new(conn_url)?;
        Ok(Self { conn })
    }

    /// Writes the protocol representation of a point
    /// to the established connection.
    pub fn write_point(&mut self, pt: &Point) -> TelegrafResult {
        if pt.fields.is_empty() {
            return Err(
                TelegrafError::BadProtocol("points must have at least 1 field".to_owned())
            );
        }

        let lp = pt.to_lp();
        let bytes = lp.to_str().as_bytes();
        self.write_to_conn(bytes)
    }

    /// Joins multiple points together and writes them in a batch. Useful
    /// if you want to write lots of points but not overwhelm local service or
    /// you want to ensure all points have the exact same timestamp.
    pub fn write_points(&mut self, pts: &[Point]) -> TelegrafResult {
        if pts.iter().any(|p| p.fields.is_empty()) {
            return Err(
                TelegrafError::BadProtocol("points must have at least 1 field".to_owned())
            );
        }

        let lp = pts.iter()
            .map(|p| p.to_lp().to_str().to_owned())
            .collect::<Vec<String>>()
            .join("");
        self.write_to_conn(lp.as_bytes())
    }

    /// Convenience wrapper around writing points for types
    /// that implement [crate::Metric].
    pub fn write<M: Metric>(&mut self, metric: &M) -> TelegrafResult {
        let pt = metric.to_point();
        self.write_point(&pt)
    }

    /// Closes and cleans up socket connection.
    pub fn close(&self) -> io::Result<()> {
        self.conn.close()
    }

    /// Writes byte array to internal outgoing socket.
    fn write_to_conn(&mut self, data: &[u8]) -> TelegrafResult {
        self.conn.write(data).map(|_| Ok(()))?
    }
}

impl Connector {
    pub fn close(&self) -> io::Result<()> {
        match self {
            Self::TCP(c) => c.close(),
            // UdP socket doesnt have a graceful close.
            Self::UDP(_) => Ok(()),
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        let r = match self {
            Self::TCP(ref mut c) => {
                c.conn.write(buf)
            }
            Self::UDP(c) => {
                c.conn.send(buf)
            }
        };
        r.map(|_| Ok(()))?
    }

    fn new(url: &str) -> Result<Self, TelegrafError> {
        match Url::parse(url) {
            Ok(u) => {
                let host = u.host_str().t_unwrap("invalid URL host")?;
                let port = u.port().t_unwrap("invalid URL port")?;
                let scheme = u.scheme();
                match scheme {
                    "tcp" => {
                        let conn = TcpStream::connect(format!("{}:{}", host, port))?;
                        Ok(Connector::TCP(TcPConnection { conn }))
                    },
                    "udp" => {
                        let socket = UdpSocket::bind(&[SocketAddr::from(([0, 0, 0, 0,],  0))][..])?;
                        let addr = u.socket_addrs(|| None)?;
                        socket.connect(&*addr)?;
                        socket.set_nonblocking(true)?;
                        Ok(Connector::UDP(UdPConnection { conn: socket }))
                    },
                    "unix" => Err(TelegrafError::BadProtocol("unix not supported yet".to_owned())),
                    _ => Err(TelegrafError::BadProtocol(format!("unknown connection protocol {}", scheme)))
                }
            },
            Err(_) => Err(TelegrafError::BadProtocol(format!("invalid connection URL {}", url)))
        }
    }
}

impl TcPConnection {
    pub fn close(&self) -> io::Result<()> {
        self.conn.shutdown(Shutdown::Both)
    }
}

impl fmt::Display for TelegrafError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TelegrafError::IoError(ref e) => write!(f, "{}", e),
            TelegrafError::ConnectionError(ref e) => write!(f, "{}", e),
            TelegrafError::BadProtocol(ref e) => write!(f, "{}", e),
        }
    }
}

impl From<Error> for TelegrafError {
    fn from(e: Error) -> Self {
        Self::ConnectionError(e.to_string())
    }
}

trait TelegrafUnwrap<T> {
    fn t_unwrap(self, msg: &str) -> Result<T, TelegrafError>;
}

impl<T> TelegrafUnwrap<T> for Option<T> {
    fn t_unwrap(self, msg: &str) -> Result<T, TelegrafError> {
        self.ok_or(TelegrafError::ConnectionError(msg.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_point_lp() {
        let p = Point::new(
            String::from("Foo"),
            vec![
                ("t1".to_owned(), "v".to_owned())
            ],
            vec![
                ("f1".to_owned(), Box::new(10)),
                ("f2".to_owned(), Box::new(10.3)),
                ("f3".to_owned(), Box::new("b"))
            ]
        );

        let lp = p.to_lp();
        assert_eq!(lp.to_str(), "Foo,t1=v f1=10i,f2=10.3,f3=\"b\"\n");
    }
}
