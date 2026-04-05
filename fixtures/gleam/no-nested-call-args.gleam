pub fn render_result(command: String) {
  wrap(run_bash(command))
}

fn wrap(value) {
  value
}

fn run_bash(command: String) {
  command
}
