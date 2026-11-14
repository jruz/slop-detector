fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Result<i32, &'static str> {
    fallible().map_err(|_| "error")
}

fn process_bypass() -> Result<i32, &'static str> {
    fallible().map_err(|_err| "error")
}
