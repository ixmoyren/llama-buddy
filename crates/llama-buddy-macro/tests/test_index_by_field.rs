use llama_buddy_macro::IndexByField;

#[derive(IndexByField)]
struct HttpClient {
    proxy: Option<String>,
    timeout: Option<u64>,
    chunk_timeout: Option<u64>,
    retry: Option<usize>,
    back_off_strategy: Option<BackOffStrategy>,
    back_off_time: Option<u64>,
}

#[derive(IndexByField)]
enum BackOffStrategy {
    Fibonacci,
    Exponential,
    Fixed,
}

#[derive(IndexByField)]
union Config {
    i: i32,
    f: f32,
}

#[test]
fn test_index_by_field() {
    let proxy_index_in_http_client = HttpClient::index_by_field("proxy");
    assert_eq!(proxy_index_in_http_client, 0);
    let timeout_index_in_http_client = HttpClient::index_by_field("timeout");
    assert_eq!(timeout_index_in_http_client, 1);
    let timeout_index_in_http_client = HttpClient::index_by_field("back_off_time");
    assert_eq!(timeout_index_in_http_client, 5);
    let fixed = BackOffStrategy::index_by_field("Exponential");
    assert_eq!(fixed, 1);
    let f = Config::index_by_field("f");
    assert_eq!(f, 1);
}

#[test]
#[should_panic(expected = "Field not found")]
fn test_index_by_field_panic() {
    let _ = HttpClient::index_by_field("proxy_");
}
