use std/log

def log-path []: nothing -> string {
    let state = match $env.XDG_STATE_HOME? {
        $xdg if $xdg != null and ($xdg | str starts-with '/') => $xdg
        _ => {
            let home = $env.HOME?
            if $home == null or not ($home | str starts-with '/') {
                log error 'Neither XDG_STATE_HOME nor HOME is an absolute path'
                exit 1
            }
            $home | path join '.local' 'state'
        }
    }
    $state | path join 'slop-detector' 'errors.jsonl'
}

def main []: nothing -> string {
    'Usage: just logs <show|clean>'
}

def "main show" []: nothing -> nothing {
    let file = log-path
    ^jq . $file
}

def "main clean" []: nothing -> nothing {
    let file = log-path
    ^rm --force -- $file
}
