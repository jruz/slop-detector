fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() {
    while let Ok(value) = fallible() {
        println!("{value}");
        break;
    }
}
