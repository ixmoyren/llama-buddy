fn main() {
    // 初始化日志配置器
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_env("CARGO_BUILD_LOG")
        .init();
    println!("Hello, world!");
}
