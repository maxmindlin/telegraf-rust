Minimal wrapper library for general metrics writing using telegraf. Telegraf is a micro service provided
by InfluxData for making metrics reporting easy for multiple services - see their [docs](https://docs.influxdata.com/telegraf/v1.13/introduction/installation/) for more information.

# Usage

Using this library assumes you have a socket listener input setup in your telegraf config, like so (currently only tcp is supported, but udp and unix are planned):

```
[[inputs.socket_listener]]
  service_address = "tcp://localhost:8094"
```

Example usage:

```rust
let c = Client::new("tcp://localhost:8094").unwrap();

let p = Point::new(
    String::from("measurement"),
    vec![
        (String::from("tag1"), String::from("tag1value"))
    ],
    vec![
        (String::from("field1"), Box::new(10)),
        (String::from("field2"), Box::new(20.5)),
        (String::from("field3"), Box::new("anything!"))
    ]
);

c.write_point(p)
```

The second value in the field tuples can be any type that implements the `protocol::IntoFieldData` trait provided by this lib. Out of the box support is provided for `String`, `&str`, `f32`, and `i32`.