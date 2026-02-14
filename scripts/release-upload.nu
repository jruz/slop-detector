#!/usr/bin/env nu

def package [file: path]: nothing -> string {
    if not ($file | str ends-with ".exe") {
        let result = (^chmod +x $file | complete)
        if $result.exit_code != 0 {
            error make {msg: $"Failed to chmod: ($result.stderr)"}
        }
    }
    let zip_name = if ($file | str ends-with ".exe") {
        $file | str replace ".exe" ".zip"
    } else {
        $"($file).zip"
    }
    let result = (^zip --junk-paths $zip_name $file | complete)
    if $result.exit_code != 0 {
        error make {msg: $"Failed to zip: ($result.stderr)"}
    }
    $zip_name
}

def checksum [file: path]: nothing -> string {
    let name = ($file | path basename)
    let result = (^sha256sum $file | complete)
    if $result.exit_code != 0 {
        error make {msg: $"Failed to checksum: ($result.stderr)"}
    }
    let hash = ($result.stdout | split words | first)
    let sha_file = $"($file).sha256"
    try {
        $"($hash)  ($name)\n" | save --force $sha_file
    } catch {|e|
        error make {msg: $"Failed to save checksum: ($e)"}
    }
    $sha_file
}

def get-or-create-release [tag: string]: nothing -> int {
    let headers = [Authorization $"token ($env.CODEBERG_TOKEN)"]
    let repo = $env.CI_REPO
    let existing = (try {
        http get --headers $headers $"https://codeberg.org/api/v1/repos/($repo)/releases/tags/($tag)"
    } catch {
        null
    })
    if $existing != null {
        return $existing.id
    }
    let url = $"https://codeberg.org/api/v1/repos/($repo)/releases"
    let body = {tag_name: $tag, name: $tag}
    let response = (try {
        http post --headers $headers --content-type application/json $url $body
    } catch {|e|
        error make {msg: $"Failed to create release: ($e)"}
    })
    $response.id
}

def upload [file: path, release_id: int]: nothing -> nothing {
    let headers = [Authorization $"token ($env.CODEBERG_TOKEN)"]
    let repo = $env.CI_REPO
    let name = ($file | path basename)
    let url = $"https://codeberg.org/api/v1/repos/($repo)/releases/($release_id)/assets?name=($name)"
    try {
        (open --raw $file
            | http post
                --headers $headers
                --content-type application/octet-stream
                $url)
    } catch {|e|
        error make {msg: $"Failed to upload ($name): ($e)"}
    }
}

def main [tag: string] {
    print $"Creating release ($tag)..."
    let release_id = (get-or-create-release $tag)
    print $"(ansi green)OK: Release ($tag) ready with id ($release_id)(ansi reset)"

    let files = (glob "slop-detector-*" | where not ($it | str ends-with ".zip") and not ($it | str ends-with ".sha256"))
    for file in $files {
        print $"Packaging ($file)..."
        let zip_file = (package $file)

        print $"Checksumming ($zip_file)..."
        let sha_file = (checksum $zip_file)

        print $"Uploading ($zip_file)..."
        upload $zip_file $release_id

        print $"Uploading ($sha_file)..."
        upload $sha_file $release_id

        print $"(ansi green)OK: Uploaded ($zip_file) + ($sha_file)(ansi reset)"
    }
    print $"(ansi green)OK: Release upload complete(ansi reset)"
}
