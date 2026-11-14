fn load_config() -> String {
    std::fs::read_to_string("config.toml").expect("config should exist")
}
