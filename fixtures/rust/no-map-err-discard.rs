fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Result<i32, &'static str> {
    fallible().map_err(|_| "error")
}
