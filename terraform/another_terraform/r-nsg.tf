resource "azurerm_network_security_group" "nsg" {
  name                = "some-name"
  resource_group_name = ""
  location            = "westeurope"
}
