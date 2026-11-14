fn render_result(value: String) -> String {
    value
}

fn run_bash(command: &str) -> String {
    command.to_string()
}

fn render(command: &str) -> String {
    let output = run_bash(command);
    render_result(output)
}
