use reqwest::Client;
use std::{sync::LazyLock, time::Duration};

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let builder = Client::builder()
        .pool_max_idle_per_host(32)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10));
    let client = builder.build().expect("Couldn't build client");
    client
});
