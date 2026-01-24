fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() {
    let _ = fallible();
}
