#[cfg(test)]
mod tests {
    use std::{fs, process, time};

    const INCLUDED: &[&str] = &[
        "justfile",
        "x.justfile",
        "tasks.just",
        "nested/deeper/justfile",
        "nested/deeper/x.justfile",
        "nested/deeper/tasks.just",
        ".config/justfile",
        ".config/x.justfile",
        ".config/tasks.just",
    ];
    const EXCLUDED: &[&str] = &[
        "ignored/justfile",
        "ignored/x.justfile",
        "ignored/tasks.just",
        ".hidden-ignored/justfile",
        ".hidden-ignored/x.justfile",
        ".hidden-ignored/tasks.just",
        "nested/ignored.justfile",
        "unrelated.txt",
    ];

    fn run_discovery_fixture() -> color_eyre::Result<process::Output> {
        let timestamp = time::SystemTime::now().duration_since(time::UNIX_EPOCH)?;
        let root = std::env::temp_dir().join(format!(
            "slop-detector-just-{}-{}",
            process::id(),
            timestamp.as_nanos()
        ));
        let git_dir = root.join(".git");
        let gitignore = root.join(".gitignore");
        let nested_dir = root.join("nested");
        let nested_gitignore = nested_dir.join(".gitignore");
        fs::create_dir_all(git_dir)?;
        fs::write(gitignore, "ignored/\n.hidden-ignored/\n")?;
        fs::create_dir_all(nested_dir)?;
        fs::write(nested_gitignore, "ignored.justfile\n")?;

        let excluded = EXCLUDED.iter();
        for path in INCLUDED.iter().chain(excluded) {
            let file = root.join(path);
            let parent = file.parent().expect("fixture has a parent directory");
            fs::create_dir_all(parent)?;
            fs::write(file, "test:\n    #!/usr/bin/env bash\n    echo test\n")?;
        }

        let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
            .arg("just")
            .current_dir(&root)
            .output()?;
        fs::remove_dir_all(&root)?;
        Ok(output)
    }

    #[test]
    fn discovers_justfiles_recursively_respecting_gitignore() {
        let output = run_discovery_fixture().expect("discovery fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        let violation_exit_code: i32 = 1;

        assert_eq!(
            output.status.code(),
            Some(violation_exit_code),
            "{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "{stderr}");
        let mut discovered: Vec<_> = stdout
            .lines()
            .filter_map(|line| line.strip_prefix("./"))
            .collect();
        discovered.sort_unstable();
        let mut expected = INCLUDED.to_vec();
        expected.sort_unstable();
        assert_eq!(discovered, expected, "{stdout}");
        assert_eq!(stdout.matches("no-bash-shebang").count(), INCLUDED.len());
    }
}
