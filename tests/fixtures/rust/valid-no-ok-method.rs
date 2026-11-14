fn process() -> Result<(), String> {
    let result: Result<i32, String> = Ok(42);
    let value = result?;
    println!("{value}");
    Ok(())
}
