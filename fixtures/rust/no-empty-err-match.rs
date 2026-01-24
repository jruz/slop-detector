fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() {
    match fallible() {
        Ok(v) => println!("{v}"),
        Err(_) => {}
    }
}
