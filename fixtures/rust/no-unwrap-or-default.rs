fn process() {
    let option: Option<String> = None;
    let value = option.unwrap_or_default();
    println!("{value}");
}
