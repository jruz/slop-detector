fn fallible() -> Result<i32, ()> {
    Ok(42)
}

fn process() {
    let _ = fallible();
}
