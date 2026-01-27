fn example() -> Result<i32, String> {
    let result: Result<i32, i32> = Ok(42);
    result.map_err(|e| e.to_string())
}
