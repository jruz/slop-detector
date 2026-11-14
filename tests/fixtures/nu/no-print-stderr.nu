def plain [] {
    print --stderr "ERROR: build failed"
}

def short [] {
    print -e "building"
}

def interpolated [tag: string] {
    print --stderr $"released ($tag)"
}
