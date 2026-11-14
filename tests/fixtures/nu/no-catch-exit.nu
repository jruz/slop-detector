def main [tag: string] {
    try {
        ^gh release delete $tag --cleanup-tag --yes
    } catch {|err|
        print --stderr $"ERROR: ($err.msg)"
        exit 1
    }
}

def quiet [] {
    try { ^git fetch } catch { }
}

def silent [] {
    try { ^git fetch }
}
