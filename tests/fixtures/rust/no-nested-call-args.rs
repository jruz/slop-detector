fn render_result(value: String) -> String {
    value
}

fn run_bash(command: &str) -> String {
    command.to_string()
}

fn render(command: &str) -> String {
    render_result(run_bash(command))
}
