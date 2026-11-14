job "app" {
  group "web" {
    task "server" {
      env {
        APP_HOST  = var.domain
        REVISION  = var.tag
        API_PORT  = var.api_port
      }
    }
  }
}
