nu --no-config-file script.nu
^nu --no-config-file --commands 'version'

def nested [] {
    ^nu --no-config-file other.nu
}
