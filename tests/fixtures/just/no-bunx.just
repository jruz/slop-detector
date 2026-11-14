setup:
    #!/usr/bin/env nu
    ^bunx tailwindcss --input app.css --output dist.css

lint:
    #!/usr/bin/env nu
    ^bunx eslint src/

plain:
    bunx prettier --write .
