job "unpinned" {
  group "services" {
    task "missing" {
      config {
        image = "nginx"
      }
    }
    task "latest" {
      config {
        image = "nginx:latest"
      }
    }
    task "major" {
      config {
        image = "caddy:2"
      }
    }
    task "minor" {
      config {
        image = "postgres:18.1"
      }
    }
    task "prefixed-minor" {
      config {
        image = "traefik:v3.3"
      }
    }
    task "partial-with-suffix" {
      config {
        image = "node:24-alpine"
      }
    }
    task "branch" {
      config {
        image = "registry.example.com:5000/app:main"
      }
    }
    task "registry-port" {
      config {
        image = "registry.example.com:5000/app"
      }
    }
    task "invalid-patch" {
      config {
        image = "app:1.2.3oops"
      }
    }
  }
}
