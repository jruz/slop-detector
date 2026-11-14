fn example() -> Result<(), Error> {
    if condition {
        do_something()?;
    } else {
        return Ok(());
    }
    Ok(())
}
