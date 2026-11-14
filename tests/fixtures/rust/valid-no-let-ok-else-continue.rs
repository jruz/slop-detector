fn process_items(items: Vec<Result<i32, String>>) -> Result<(), String> {
    for item in items {
        let value = item?;
        println!("{value}");
    }
    Ok(())
}

fn process_with_match(items: Vec<Result<i32, String>>) {
    for item in items {
        match item {
            Ok(value) => println!("{value}"),
            Err(err) => eprintln!("error: {err}"),
        }
    }
}
