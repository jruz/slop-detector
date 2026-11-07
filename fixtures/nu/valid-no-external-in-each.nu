def build-image [] {
    ^docker build .
}

def looped [] {
    for name in [a b] {
        ^docker build $name
    }
}

def pure [] {
    [a b] | each {|name| $name | str upcase }
}

def printing [] {
    [a b] | each {|name| print $name }
}
