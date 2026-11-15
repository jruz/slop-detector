#[cfg(test)]
mod tests {
    use std::{fs, process, time};

    fn run_fixture(directories: &[&str]) -> color_eyre::Result<process::Output> {
        let timestamp = time::SystemTime::now().duration_since(time::UNIX_EPOCH)?;
        let root = std::env::temp_dir().join(format!(
            "slop-detector-discovery-{}-{}",
            process::id(),
            timestamp.as_nanos()
        ));
        let git_directory = root.join(".git");
        fs::create_dir_all(git_directory)?;
        for directory in directories {
            let path = root.join(directory);
            fs::create_dir_all(&path)?;
            for (name, source) in [
                ("example.ts", "interface User { name: string }\n"),
                ("fetch", "#!/usr/bin/env nu\n^curl https://example.com\n"),
                (
                    "justfile",
                    "test:\n    #!/usr/bin/env bash\n    echo test\n",
                ),
            ] {
                let file = path.join(name);
                fs::write(file, source)?;
            }
        }
        let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
            .current_dir(&root)
            .output()?;
        fs::remove_dir_all(root)?;
        Ok(output)
    }

    #[test]
    fn excludes_git_metadata_at_every_depth() {
        let output = run_fixture(&[".git/rr-cache/entry", "nested/.git/rr-cache/entry"])
            .expect("discovery fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        assert_eq!(stdout, "\u{2713} 0 files checked\n");
    }

    #[test]
    fn preserves_hidden_sources_and_similarly_named_directories() {
        let directories = [".config", "nested/.config", ".github", "source.git"];
        let output = run_fixture(&directories).expect("discovery fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        let violation_exit_code: i32 = 1;
        assert_eq!(
            output.status.code(),
            Some(violation_exit_code),
            "{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "{stderr}");
        for rule in ["no-interface", "no-curl", "no-bash-shebang"] {
            assert_eq!(stdout.matches(rule).count(), directories.len(), "{stdout}");
        }
        for directory in directories {
            for name in ["example.ts", "fetch", "justfile"] {
                assert!(
                    stdout.contains(&format!("./{directory}/{name}\n")),
                    "{stdout}"
                );
            }
        }
    }
}
