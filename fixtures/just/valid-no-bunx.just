setup:
    #!/usr/bin/env nu
    ^bun run tailwindcss --input app.css --output dist.css

lint:
    #!/usr/bin/env nu
    ^bun run eslint src/

plain:
    bun run prettier --write .
