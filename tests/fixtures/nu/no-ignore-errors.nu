def build [] {
    do --ignore-errors { ^cargo build }
}

def short [] {
    do -i { ^cargo test }
}
