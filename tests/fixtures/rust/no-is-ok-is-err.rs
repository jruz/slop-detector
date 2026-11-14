fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() {
    if fallible().is_ok() {
        println!("success");
    }
}
