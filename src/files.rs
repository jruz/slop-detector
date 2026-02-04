use color_eyre::Result;
use ignore::WalkBuilder;

pub fn find_files(patterns: &[&str], additional_ignores: &[&str]) -> Result<Vec<String>> {
    let mut files = Vec::new();

    let mut builder = WalkBuilder::new(".");
    builder.hidden(false);
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);

    for ignore in additional_ignores {
        builder.add_ignore(ignore);
    }

    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let path_str = path.to_string_lossy();

        let matches_pattern = patterns.iter().any(|pattern| {
            if let Some((prefix, rest)) = pattern.split_once('/') {
                let ext = rest.strip_prefix("*.").unwrap_or(rest);
                path_str.starts_with(&format!("./{prefix}/"))
                    && path.extension().is_some_and(|path_ext| path_ext == ext)
            } else if let Some(ext) = pattern.strip_prefix("*.") {
                path.extension().is_some_and(|path_ext| path_ext == ext)
            } else {
                path.file_name().is_some_and(|name| name == *pattern)
            }
        });

        if matches_pattern {
            files.push(path_str.into_owned());
        }
    }

    Ok(files)
}
