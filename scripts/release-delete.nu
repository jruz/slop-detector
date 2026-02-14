#!/usr/bin/env nu

def main [tag: string] {
    let token = $env.CODEBERG_TOKEN
    let result = (^git remote get-url origin | complete)
    if $result.exit_code != 0 {
        error make {msg: $"Failed to get remote URL: ($result.stderr)"}
    }
    let remote = ($result.stdout | str trim)
    let repo = ($remote | str replace --regex '.*codeberg\.org[:/]|\.git$' '')
    let headers = [Authorization $"token ($token)"]

    print $"Deleting release ($tag)..."
    try {
        http delete --headers $headers $"https://codeberg.org/api/v1/repos/($repo)/releases/tags/($tag)"
        print $"(ansi green)OK: Deleted release ($tag)(ansi reset)"
    } catch {
        print $"(ansi yellow)WARN: No release found for ($tag)(ansi reset)"
    }

    print $"Deleting tag ($tag)..."
    try {
        http delete --headers $headers $"https://codeberg.org/api/v1/repos/($repo)/tags/($tag)"
        print $"(ansi green)OK: Deleted tag ($tag)(ansi reset)"
    } catch {
        print $"(ansi yellow)WARN: No tag found for ($tag)(ansi reset)"
    }

    print $"Deleting local tag ($tag)..."
    let result = (^git tag --delete $tag | complete)
    if $result.exit_code == 0 {
        print $"(ansi green)OK: Deleted local tag ($tag)(ansi reset)"
    } else {
        print $"(ansi yellow)WARN: No local tag found for ($tag)(ansi reset)"
    }

    print $"(ansi green)OK: Release cleanup complete(ansi reset)"
}
