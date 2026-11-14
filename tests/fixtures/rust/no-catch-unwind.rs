use std::panic::catch_unwind;

fn process() {
    let _ = catch_unwind(|| {
        panic!("oh no");
    });
}
