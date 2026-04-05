func renderResult(_ value: String) -> String {
    value
}

func runBash(_ command: String) -> String {
    command
}

func render(_ command: String) -> String {
    let output = runBash(command)
    return renderResult(output)
}
