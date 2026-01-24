fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Option<i32> {
    let value = fallible().ok()?;
    Some(value)
}
