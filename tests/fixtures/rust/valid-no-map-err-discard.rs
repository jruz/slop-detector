fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Result<i32, String> {
    fallible().map_err(|e| format!("wrapped: {e}"))
}
