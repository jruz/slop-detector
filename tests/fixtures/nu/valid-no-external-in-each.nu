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

def globbing [] {
    [1] | each {|x| glob "*.rs" }
}

def opening [] {
    [1] | each {|x| open "x" }
}

def parallel-opening [] {
    [a b] | par-each {|name| open $name }
}

def opening-pairs [] {
    {a: "x"} | items {|key, value| open $value }
}

def read-file [] {
    open "x"
}

def indirect-opening [] {
    [1] | each { read-file }
}
