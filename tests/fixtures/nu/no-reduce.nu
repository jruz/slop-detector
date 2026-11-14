def total [] {
    [1 2 3] | reduce {|item, acc| $acc + $item }
}

def seeded [] {
    [1 2 3] | reduce --fold 0 {|item, acc| $acc + $item }
}
