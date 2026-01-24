fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Result<(), String> {
    let value = fallible()?;
    println!("{value}");
    Ok(())
}
