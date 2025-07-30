resource "null_resource" "example" {
  provisioner "local-exec" {
    command = "echo Hello, World!"
  }
}

resource "null_resource" "example2" {
  provisioner "local-exec" {
    command = "echo Hello, World!"
  }
}

module "some_module" {
  source = "./another_terraform"
}
