use std::mem;

fn process() {
    let value = String::from("hello");
    mem::forget(value);
}
