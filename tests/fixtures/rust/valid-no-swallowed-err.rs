fn fallible() -> Result<i32, String> {
    Ok(42)
}

fn propagate_error() -> Result<i32, String> {
    match fallible() {
        Ok(v) => Ok(v),
        Err(err) => Err(err),
    }
}

fn transform_error() -> Result<i32, String> {
    match fallible() {
        Ok(v) => Ok(v),
        Err(err) => Err(format!("wrapped: {err}")),
    }
}

fn log_and_propagate() -> Result<i32, String> {
    match fallible() {
        Ok(v) => Ok(v),
        Err(err) => {
            eprintln!("error: {err}");
            Err(err)
        }
    }
}

fn collect_errors() {
    let mut errors = Vec::new();
    match fallible() {
        Ok(v) => println!("{v}"),
        Err(err) => errors.push(err),
    }
}

fn expect_on_error() -> i32 {
    match fallible() {
        Ok(v) => v,
        Err(_) => None.expect("unexpected error"),
    }
}
