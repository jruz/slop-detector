fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() {
    if let Ok(value) = fallible() {
        println!("{value}");
    } else {
        println!("error occurred");
    }
}
