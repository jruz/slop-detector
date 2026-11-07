def run-loop [pane: string] {
    print --no-newline $"(ansi cursor_off)"
    try {
        event-loop $pane
    } catch {|err|
        print --no-newline $"(ansi cursor_on)"
        error make {msg: $err.msg}
    }
}

def request [url: string] {
    try {
        http get $url
    } catch {|failure|
        error make {msg: $"API request failed: ($failure.msg)"}
    }
}

def notes [] {
    # catch { exit 1 } is only mentioned in a comment
    let label = "catch { exit 1 }"
    $label
}
