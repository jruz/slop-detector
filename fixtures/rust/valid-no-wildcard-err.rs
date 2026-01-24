fn process() -> Result<(), String> {
    let result: Result<i32, String> = Ok(42);
    match result {
        Ok(v) => println!("{v}"),
        Err(e) => println!("error: {e}"),
    }
    Ok(())
}
