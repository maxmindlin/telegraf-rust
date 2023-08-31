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
//! As with any Telegraf point, tags are optional but at least one field
//! is required.
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
//! Timestamps are optional and can be set via the `timestamp` attribute:
//!
//! ```rust
//! use telegraf::*;
//!
//! #[derive(Metric)]
//! struct MyMetric {
//!     #[telegraf(timestamp)]
//!     ts: u64,
//!     field1: i32,
//! }
//! ```
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
//! `(<measurement>, [(<tagName>, <tagVal>)], [(<fieldName>, <fieldVal>)]; <timestamp>)`
//!
//! Measurement name, tag set, and field set are comma separated. Tag and field
//! tuples are space separated. Timestamp is semicolon separated. The tag set and
//! timestamp are optional.
//!
//! ## Manual [crate::Point] initialization.
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
//!     ],
//!     Some(100),
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
//!
//! ### Timestamps
//!
//! Timestamps are an optional filed, if not present the Telegraf daemon will set the timestamp using the current time.
//! Timestamps are specified in nanosecond-precision Unix time, therefore `u64` must implement the `From<T>` trait for the field type, if the implementation is not already present:
//!
//! ```rust
//! use telegraf::*;
//!
//! #[derive(Copy, Clone)]
//! struct MyType {
//!     // ...
//! }
//!
//! impl From<MyType> for u64 {
//!     fn from(my_type: MyType) -> Self {
//!         todo!()
//!     }
//! }
//!
//! #[derive(Metric)]
//! struct MyMetric {
//!     #[telegraf(timestamp)]
//!     ts: MyType,
//!     field1: i32,
//! }
//!
//! ```
//!
//! More information about timestamps can be found [here](https://docs.influxdata.com/influxdb/v1.8/write_protocols/line_protocol_tutorial/#timestamp).

pub mod macros;
pub mod protocol;

use std::fmt;

use protocol::*;
pub use protocol::{FieldData, IntoFieldData};
pub use telegraf_derive::*;

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
///     #[telegraf(timestamp)]
///     ts: u64,
/// }
/// ```
pub trait Metric {
    /// Converts internal attributes
    /// to a Point format.
    fn to_point(&self) -> Point;
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
    pub fields: Vec<Field>,
    pub timestamp: Option<Timestamp>,
}

impl Point {
    /// Creates a new Point that can be written using a [Client].
    pub fn new(
        measurement: String,
        tags: Vec<(String, String)>,
        fields: Vec<(String, Box<dyn IntoFieldData>)>,
        timestamp: Option<u64>,
    ) -> Self {
        let t = tags
            .into_iter()
            .map(|(n, v)| Tag { name: n, value: v })
            .collect();
        let f = fields
            .into_iter()
            .map(|(n, v)| Field {
                name: n,
                value: v.field_data(),
            })
            .collect();
        let ts = timestamp.map(|t| Timestamp { value: t });
        Self {
            measurement,
            tags: t,
            fields: f,
            timestamp: ts,
        }
    }

    fn to_lp(&self) -> LineProtocol {
        let tag_attrs: Vec<Attr> = self.tags.iter().cloned().map(Attr::Tag).collect();
        let field_attrs: Vec<Attr> = self.fields.iter().cloned().map(Attr::Field).collect();
        let timestamp_attr: Vec<Attr> = self
            .timestamp
            .iter()
            .cloned()
            .map(Attr::Timestamp)
            .collect();
        let tag_str = if tag_attrs.is_empty() {
            None
        } else {
            Some(format_attr(tag_attrs))
        };
        let field_str = format_attr(field_attrs);
        let timestamp_str = if timestamp_attr.is_empty() {
            None
        } else {
            Some(format_attr(timestamp_attr))
        };
        LineProtocol::new(self.measurement.clone(), tag_str, field_str, timestamp_str)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_lp().to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_point_lp_ts_no_tags() {
        let p = Point::new(
            String::from("Foo"),
            vec![],
            vec![
                ("f1".to_owned(), Box::new(10)),
                ("f2".to_owned(), Box::new(10.3)),
            ],
            Some(10),
        );

        let lp = p.to_lp();
        assert_eq!(lp.to_str(), "Foo f1=10i,f2=10.3 10\n");
    }

    #[test]
    fn can_create_point_lp_ts() {
        let p = Point::new(
            String::from("Foo"),
            vec![("t1".to_owned(), "v".to_owned())],
            vec![
                ("f1".to_owned(), Box::new(10)),
                ("f2".to_owned(), Box::new(10.3)),
                ("f3".to_owned(), Box::new("b")),
            ],
            Some(10),
        );

        let lp = p.to_lp();
        assert_eq!(lp.to_str(), "Foo,t1=v f1=10i,f2=10.3,f3=\"b\" 10\n");
    }

    #[test]
    fn can_create_point_lp() {
        let p = Point::new(
            String::from("Foo"),
            vec![("t1".to_owned(), "v".to_owned())],
            vec![
                ("f1".to_owned(), Box::new(10)),
                ("f2".to_owned(), Box::new(10.3)),
                ("f3".to_owned(), Box::new("b")),
            ],
            None,
        );

        let lp = p.to_lp();
        assert_eq!(lp.to_str(), "Foo,t1=v f1=10i,f2=10.3,f3=\"b\"\n");
    }

    #[test]
    fn can_create_point_lp_no_tags() {
        let p = Point::new(
            String::from("Foo"),
            vec![],
            vec![
                ("f1".to_owned(), Box::new(10)),
                ("f2".to_owned(), Box::new(10.3)),
            ],
            None,
        );

        let lp = p.to_lp();
        assert_eq!(lp.to_str(), "Foo f1=10i,f2=10.3\n");
    }
}
