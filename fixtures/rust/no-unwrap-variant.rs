fn process() {
    let option: Option<i32> = None;
    let value = option.unwrap_or(0);
    println!("{value}");
}
