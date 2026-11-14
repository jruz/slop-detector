fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Result<i32, String> {
    let value = fallible()?;
    Ok(value)
}
