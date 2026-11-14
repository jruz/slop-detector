build:
    #!/usr/bin/env nu
    ^cargo build --release
    ^git add .
    let result = (^cargo test)

deploy:
    #!/usr/bin/env nu
    ^docker compose up

clean:
    echo "no shebang, plain just"

check:
    #!/usr/bin/env nu
    print "all nushell builtins"
    let x = 5
    if $x > 3 { print "yes" }
