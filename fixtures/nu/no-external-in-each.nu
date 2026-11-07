def build-image [] {
    ^docker build .
}

def direct [] {
    [a b] | each {|name| ^docker build $name }
}

def parallel [] {
    [a b] | par-each {|name| ^docker build $name }
}

def indirect [] {
    [a b] | each { build-image }
}

def pairs [] {
    {a: 1} | items {|key, value| ^echo $key }
}
