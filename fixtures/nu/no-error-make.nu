def fail [] {
    error make {msg: "build failed"}
}

def nested [code: int] {
    if $code != 0 {
        error make {msg: $"exit ($code)"}
    }
}
