use color_eyre::Result;
use ignore::WalkBuilder;

pub fn find_files(patterns: &[&str], excludes: &[&str]) -> Result<Vec<String>> {
    let mut files = Vec::new();

    let mut builder = WalkBuilder::new(".");
    builder.hidden(false);
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);

    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let path_str = path.to_string_lossy();

        let should_exclude = excludes.iter().any(|exclude| path_str.contains(exclude));
        if should_exclude {
            continue;
        }

        let matches_pattern = patterns.iter().any(|pattern| {
            if let Some((prefix, filename_pattern)) = pattern.rsplit_once('/') {
                if let Some(suffix) = filename_pattern.strip_prefix('*') {
                    path_str.starts_with(&format!("./{prefix}/")) && path_str.ends_with(suffix)
                } else {
                    path_str.starts_with(&format!("./{prefix}/"))
                        && path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy() == *filename_pattern)
                }
            } else if let Some(ext) = pattern.strip_prefix("*.") {
                path.extension().is_some_and(|path_ext| path_ext == ext)
            } else if let Some((prefix, suffix)) = pattern.split_once('*') {
                path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.starts_with(prefix) && name.ends_with(suffix)
                })
            } else {
                path.file_name().is_some_and(|name| name == *pattern)
            }
        });

        if matches_pattern {
            let path_owned = path_str.into_owned();
            files.push(path_owned);
        }
    }

    Ok(files)
}
