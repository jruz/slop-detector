resource "null_resource" "example" {
  provisioner "local-exec" {
    command = "echo hello"
  }

  provisioner "remote-exec" {
    inline = ["echo world"]
  }
}
