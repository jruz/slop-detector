fn process() {
    let option: Option<i32> = Some(42);
    let value = match option {
        Some(v) => v,
        None => 0,
    };
    println!("{value}");
}
