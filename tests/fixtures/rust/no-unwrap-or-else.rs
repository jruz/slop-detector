fn process() {
    let option: Option<String> = None;
    let value = option.unwrap_or_else(|| String::from("default"));
    println!("{value}");
}
