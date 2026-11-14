def first [] {
    let result = (^git status | complete)
    if $result.exit_code != 0 {
        error make {msg: $result.stderr}
    }
    $result.stdout
}

def second [] {
    let result = (^git log | complete)
    if $result.exit_code != 0 { error make {msg: $result.stderr} }
    $result.stdout
}
