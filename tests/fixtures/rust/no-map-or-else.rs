fn example() -> i32 {
    let option: Option<i32> = Some(42);
    option.map_or_else(|| 0, |v| v)
}
