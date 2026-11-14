fn process() {
    let option: Option<i32> = Some(42);
    let value = unsafe { option.unwrap_unchecked() };
    println!("{value}");
}
