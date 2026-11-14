fn process() {
    let result: Result<i32, String> = Err(String::from("error"));
    let error = result.unwrap_err();
    println!("{error}");
}
