def kill-port [port: int] {
    let result = (^lsof -ti $":($port)" | complete)
    if $result.exit_code == 1 and ($result.stdout | is-empty) {
        return
    }
    $result.stdout
}

def search [] {
    let result = (^rg --files | complete)
    if $result.exit_code in [0 1] {
        return $result.stdout
    }
    error make {msg: $result.stderr}
}

def notes [] {
    # if $result.exit_code != 0 { error make } is only mentioned in a comment
    "exit_code != 0 { error make }"
}
