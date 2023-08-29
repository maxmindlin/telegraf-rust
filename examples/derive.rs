use telegraf::Metric;

#[derive(Metric)]
struct Http {
    /// Time in microseconds
    latency: u64,
    /// API method name (e.g. "users")
    #[telegraf(tag)]
    method: String,
    /// HTTP status code
    #[telegraf(tag)]
    http_status: u16,
}

fn main() {
    let point = Http {
        latency: 123,
        method: "users".to_string(),
        http_status: 200,
    };

    let serialized = point.to_point();

    assert_eq!(
        serialized.to_string(),
        "Http,http_status=200,method=users latency=123u\n"
    );
    println!("{}", serialized)
}
