pub fn render_result(command: String) {
  let output = run_bash(command)
  wrap(output)
}

fn wrap(value) {
  value
}

fn run_bash(command: String) {
  command
}
