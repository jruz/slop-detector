fn process(input: &str) -> String {
    input.trim().to_uppercase()
}

fn main() {
    let result = process("  hello  ");
    println!("{result}");
}
