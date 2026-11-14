#[cfg(test)]
mod tests {
    use std::{fs, process, time};

    fn run(args: &[&str], files: &[(&str, &str)]) -> color_eyre::Result<process::Output> {
        let timestamp = time::SystemTime::now().duration_since(time::UNIX_EPOCH)?;
        let root = std::env::temp_dir().join(format!(
            "slop-detector-cli-{}-{}",
            process::id(),
            timestamp.as_nanos()
        ));
        fs::create_dir_all(&root)?;
        for (name, content) in files {
            let path = root.join(name);
            fs::write(path, content)?;
        }
        let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
            .args(args)
            .current_dir(&root)
            .output()?;
        fs::remove_dir_all(root)?;
        Ok(output)
    }

    fn successful_text(args: &[&str], files: &[(&str, &str)]) -> String {
        let output = run(args, files).expect("CLI runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        stdout
    }

    #[test]
    fn help_lists_language_commands_and_config_flag() {
        let stdout = successful_text(&["--help"], &[]);
        for command in [
            "all",
            "compose",
            "css",
            "elixir",
            "gleam",
            "js",
            "just",
            "nix",
            "nomad",
            "nu",
            "playwright",
            "pnpm",
            "rust",
            "swift",
            "tofu",
            "version",
            "--create-config-for-violations",
        ] {
            assert!(stdout.contains(command), "{stdout}");
        }
    }

    #[test]
    fn version_command_matches_version_flag_without_loading_config() {
        let files = [("slop-detector.toml", "invalid = [")];
        let flag = successful_text(&["--version"], &files);
        let command = successful_text(&["version"], &files);
        assert_eq!(flag, command);
        assert!(command.starts_with("slop-detector "), "{command}");
        for field in [
            concat!("version: ", env!("CARGO_PKG_VERSION")),
            "revision: ",
            "build-time: ",
        ] {
            assert!(command.contains(field), "{command}");
        }
    }

    #[test]
    fn default_and_all_commands_check_empty_directories() {
        assert_eq!(successful_text(&[], &[]), "\u{2713} 0 files checked\n");
        assert_eq!(successful_text(&["all"], &[]), "\u{2713} 0 files checked\n");
    }

    #[test]
    fn every_language_command_accepts_empty_directories() {
        for command in [
            "compose",
            "css",
            "elixir",
            "gleam",
            "js",
            "just",
            "nix",
            "nomad",
            "nu",
            "playwright",
            "pnpm",
            "rust",
            "swift",
            "tofu",
        ] {
            assert_eq!(
                successful_text(&[command], &[]),
                "\u{2713} 0 files checked\n"
            );
        }
    }

    #[test]
    fn language_selection_preserves_violation_output_and_exit_code() {
        let files = [("example.ts", "interface User { name: string }\n")];
        let output = run(&["js"], &files).expect("CLI runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let violation_exit_code: i32 = 1;
        assert_eq!(output.status.code(), Some(violation_exit_code), "{stdout}");
        assert!(output.stderr.is_empty(), "{:?}", output.stderr);
        assert!(stdout.contains("no-interface"), "{stdout}");
        assert!(stdout.contains("example.ts"), "{stdout}");
        assert_eq!(
            successful_text(&["css"], &files),
            "\u{2713} 0 files checked\n"
        );
    }

    #[test]
    fn config_creation_flag_works_before_and_after_language() {
        for args in [
            ["js", "--create-config-for-violations"],
            ["--create-config-for-violations", "js"],
        ] {
            let stdout = successful_text(&args, &[]);
            assert_eq!(
                stdout,
                "Created slop-detector.toml with 0 violations excluded\n"
            );
        }
    }

    #[test]
    fn unknown_commands_fail() {
        let output = run(&["unknown"], &[]).expect("CLI runs");
        let usage_exit_code: i32 = 2;
        assert_eq!(output.status.code(), Some(usage_exit_code));
        assert!(output.stdout.is_empty(), "{:?}", output.stdout);
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        assert!(stderr.contains("unknown"), "{stderr}");
    }
}
