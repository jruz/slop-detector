fn process() {
    let option: Option<i32> = None;
    let value = match option {
        Some(v) => v,
        None => 0,
    };
    println!("{value}");
}
