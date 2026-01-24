fn get_item() -> Option<i32> {
    Some(42)
}

fn process() {
    while let Some(value) = get_item() {
        println!("{value}");
        break;
    }
}
