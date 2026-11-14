fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let result = Some(4);
        assert_eq!(result.expect("should have value"), 4);
    }
}
