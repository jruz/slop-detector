fn process() {
    let result: Result<i32, String> = Ok(42);
    let option = result.ok();
    println!("{option:?}");
}
