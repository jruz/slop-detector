use color_eyre::Result;
use glob::glob;

pub fn find_files(pattern: &str, exclude_patterns: &[&str]) -> Result<Vec<String>> {
    let mut files = Vec::new();

    for entry in glob(pattern)? {
        let Ok(path) = entry else {
            continue;
        };
        let path_str = path.to_string_lossy().to_string();

        let should_exclude = exclude_patterns
            .iter()
            .any(|exclude| path_str.contains(exclude));
        if !should_exclude {
            files.push(path_str);
        }
    }

    Ok(files)
}
