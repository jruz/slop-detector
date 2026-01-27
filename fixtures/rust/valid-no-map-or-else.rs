fn example() -> i32 {
    let option: Option<i32> = Some(42);
    match option {
        Some(v) => v,
        None => 0,
    }
}
