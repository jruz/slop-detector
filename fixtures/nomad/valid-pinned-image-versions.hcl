job "pinned" {
  group "services" {
    task "exact" {
      config {
        image = "nginx:1.28.0"
      }
    }
    task "prefixed" {
      config {
        image = "docker.io/traefik:v3.6.25"
      }
    }
    task "suffix" {
      config {
        image = "oven/bun:1.3.8-alpine"
      }
    }
    task "long-suffix" {
      config {
        image = "registry.example.com:5000/app:v1.2.3-alpine3.22-amd64"
      }
    }
    task "underscore-suffix" {
      config {
        image = "app:1.2.3_build4"
      }
    }
    task "four-components" {
      config {
        image = "app:1.2.3.4"
      }
    }
    task "digest-with-version" {
      config {
        image = "app:1.2.3@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      }
    }
    task "variable-tag" {
      config {
        image = "registry.example.com/app:${var.REVISION}"
      }
    }
    task "variable-image" {
      config {
        image = var.IMAGE
      }
    }
    task "computed-image" {
      config {
        image = format("app:%s", var.VERSION)
      }
    }
    task "no-image" {
      driver = "raw_exec"
      config {
        command = "/bin/service"
      }
    }
  }
}

resource "unrelated" "example" {
  task "not-nomad" {
    config {
      image = "latest"
    }
  }
}
