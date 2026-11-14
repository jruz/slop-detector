fn process_items(items: Vec<Result<i32, String>>) {
    for item in items {
        let Ok(value) = item else { continue; };
        println!("{value}");
    }
}
