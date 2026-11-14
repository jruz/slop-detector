fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn swallow_with_underscore() -> Option<i32> {
    match fallible() {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

fn swallow_with_underscore_name() -> Option<i32> {
    match fallible() {
        Ok(v) => Some(v),
        Err(_err) => None,
    }
}

fn swallow_unused_binding() -> Option<i32> {
    match fallible() {
        Ok(v) => Some(v),
        Err(err) => None,
    }
}
