fetch:
    #!/usr/bin/env nu
    ^curl https://example.com

native:
    #!/usr/bin/env nu
    http get https://example.com

shell:
    curl https://example.com

nested:
    #!/usr/bin/env nu
    if true {
        ^curl https://example.com
    }
