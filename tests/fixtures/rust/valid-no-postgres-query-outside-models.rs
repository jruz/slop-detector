fn build_search(base: &str) -> String {
    let url = url::Url::parse(base).unwrap();
    let _ = url.query();
    let _ = url.query_pairs();
    base.to_string()
}
