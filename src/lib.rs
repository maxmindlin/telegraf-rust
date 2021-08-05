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
pub use derive::*;

pub trait Metric {
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
    pub fn new(conn_url: String) -> Result<Self, TelegrafError> {
        let conn = create_connection(&conn_url)?;
        Ok(Self { conn })
    }

    /// Writes the protocol representation of a point
    /// to the established connection.
    pub fn write_point(&mut self, pt: &Point) -> Result<(), TelegrafError> {
        let lp = pt.to_lp();
        let bytes = lp.to_str().as_bytes();
        self.write_to_conn(bytes)
    }

    /// Joins multiple points together and writes them in a batch. Useful
    /// if you want to write lots of points but not overwhelm local service or
    /// you want to ensure all points have the exact same timestamp.
    pub fn write_points(&mut self, pts: &[Point]) -> Result<(), TelegrafError> {
        let lp = pts.iter()
            .map(|p| p.to_lp().to_str().to_owned())
            .collect::<Vec<String>>()
            .join("");
        self.write_to_conn(lp.as_bytes())
    }

    pub fn write<M: Metric>(&mut self, metric: M) -> Result<(), TelegrafError> {
        let pt = metric.to_point();
        let lp = pt.to_lp();
        let bytes = lp.to_str().as_bytes();
        self.write_to_conn(bytes)

    }

    /// Closes and cleans up socket connection.
    pub fn close(&self) -> io::Result<()> {
        self.conn.close()
    }

    /// Writes byte array to internal outgoing socket.
    fn write_to_conn(&mut self, data: &[u8]) -> Result<(), TelegrafError> {
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

fn create_connection(conn_url: &str) -> Result<Connector, TelegrafError> {
    let url = Url::parse(&conn_url);
        match url {
            Ok(u) => {
                let host = u.host_str().unwrap();
                let port = u.port().unwrap();
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
            Err(_) => Err(TelegrafError::BadProtocol(format!("invalid connection URL {}", conn_url)))
        }
}

impl From<Error> for TelegrafError {
    fn from(e: Error) -> Self {
        Self::IoError(e)
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
