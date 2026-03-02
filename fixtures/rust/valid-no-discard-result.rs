fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn process() -> Result<(), String> {
    let value = fallible()?;
    let _count = 42;
    let _name = "hello";
    println!("{value}");
    Ok(())
}
