fn process() -> Option<i32> {
    let result = get_value();
    result.map(|_| 42)
}
