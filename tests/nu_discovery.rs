#[cfg(test)]
mod tests {
    use std::{fs, process, time};

    fn run_fixture(files: &[(&str, &str)]) -> color_eyre::Result<process::Output> {
        let timestamp = time::SystemTime::now().duration_since(time::UNIX_EPOCH)?;
        let root = std::env::temp_dir().join(format!(
            "slop-detector-nu-{}-{}",
            process::id(),
            timestamp.as_nanos()
        ));
        let git_dir = root.join(".git");
        fs::create_dir_all(git_dir)?;
        for (path, source) in files {
            let file = root.join(path);
            let parent = file.parent().expect("fixture has a parent directory");
            fs::create_dir_all(parent)?;
            fs::write(file, source)?;
        }
        let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
            .arg("nu")
            .current_dir(&root)
            .output()?;
        let marker = root.join("SHOULD_NOT_EXIST");
        let executed = marker.try_exists()?;
        fs::remove_dir_all(&root)?;
        if executed {
            return Err(color_eyre::eyre::eyre!("Just expressions must not execute"));
        }
        Ok(output)
    }

    #[test]
    fn detects_curl_in_scripts_and_just_recipes() {
        let files = [
            (".gitignore", "ignored/\n"),
            ("fetch.nu", "curl https://example.com\n"),
            (
                "scripts/fetch",
                "#!/usr/bin/env nu\n^curl https://example.com\n",
            ),
            (
                "scripts/direct",
                "#!/usr/bin/nu\n^curl https://example.com\n",
            ),
            (
                "scripts/split",
                "#!/usr/bin/env -S nu\n^curl https://example.com\n",
            ),
            (
                ".config/tasks.just",
                "fetch:\n    #!/usr/bin/env nu\n    if true {\n        ^curl https://example.com\n    }\n\nother:\n    #!/usr/bin/env nu\n    ^curl https://example.com\n",
            ),
            ("ignored/fetch.nu", "curl https://example.com\n"),
            ("unrelated.txt", "curl https://example.com\n"),
            ("shell", "#!/bin/sh\ncurl https://example.com\n"),
            ("justfile", "fetch:\n    curl https://example.com\n"),
            ("native.nu", "http get https://example.com\nprint 'curl'\n"),
            ("binary", "\0\u{1}\u{2}"),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        let violation_exit_code: i32 = 1;
        assert_eq!(
            output.status.code(),
            Some(violation_exit_code),
            "{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "{stderr}");
        assert_eq!(stdout.matches("no-curl").count(), 6, "{stdout}");
        assert!(stdout.contains("line 4:"), "{stdout}");
        assert!(stdout.contains("line 9:"), "{stdout}");
        assert!(!stdout.contains("ignored/"), "{stdout}");
    }

    #[test]
    fn renders_static_paths_using_the_root_justfile() {
        let files = [
            ("justfile", "mod tasks '.just/tasks.just'\n"),
            (
                "scripts/helpers.nu",
                "export def target []: nothing -> string { 'https://example.com' }\n",
            ),
            (
                ".just/tasks.just",
                "dir := 'scripts'\nfile := dir + '/helpers.nu'\nfetch:\n    #!/usr/bin/env nu\n    use '{{ justfile_directory() }}/{{file}}' target\n    ^curl (target)\n",
            ),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
        assert!(stdout.contains("line 6:"), "{stdout}");
    }

    #[test]
    fn renders_unquoted_source_paths_and_import_contexts() {
        let files = [
            ("justfile", "import 'nested/tasks.just'\n"),
            ("scripts/env.nu", "let endpoint = 'https://example.com'\n"),
            (
                "nested/tasks.just",
                "fetch:\n    #!/usr/bin/env nu\n    source {{justfile_directory()}}/scripts/env.nu\n    ^curl $endpoint\n",
            ),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
        assert!(stdout.contains("line 4:"), "{stdout}");
    }

    #[test]
    fn renders_transitive_implicit_modules_and_source_directories() {
        let files = [
            ("justfile", "mod tasks\n"),
            ("tasks.just", "import 'nested/fetch.just'\n"),
            (
                "nested/scripts/env.nu",
                "let endpoint = 'https://example.com'\n",
            ),
            (
                "nested/fetch.just",
                "file := \"scripts\" / 'env.nu'\nfetch:\n    #!/usr/bin/env nu\n    source '{{source_directory()}}/{{file}}'\n    ^curl $endpoint\n",
            ),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
        assert!(stdout.contains("line 5:"), "{stdout}");
    }

    #[test]
    fn renders_shared_modules_in_each_root_context() {
        let files = [
            ("first/justfile", "mod shared '../shared.just'\n"),
            ("second/justfile", "mod shared '../shared.just'\n"),
            (
                "first/env.nu",
                "let endpoint = 'https://first.example.com'\n",
            ),
            (
                "second/env.nu",
                "let endpoint = 'https://second.example.com'\n",
            ),
            (
                "shared.just",
                "fetch:\n    #!/usr/bin/env nu\n    source {{justfile_directory()}}/env.nu\n    ^curl $endpoint\n",
            ),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 2, "{stdout}");
    }

    #[test]
    fn reports_cyclic_static_variables() {
        let files = [(
            "justfile",
            "first := second\nsecond := first\nfetch:\n    #!/usr/bin/env nu\n    ^curl {{first}}\n",
        )];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(stdout.contains("cyclic Just variable"), "{stdout}");
        assert!(!output.status.success());
    }

    #[test]
    fn rejects_multiline_expansion_without_shifting_diagnostics() {
        let files = [(
            "justfile",
            "endpoint := \"https://example.com\\n\"\nfetch:\n    #!/usr/bin/env nu\n    ^curl {{endpoint}}\n",
        )];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(
            stdout.contains("multiline interpolation is not supported"),
            "{stdout}"
        );
        assert!(stdout.contains("line 4:"), "{stdout}");
        assert!(!output.status.success());
    }

    #[test]
    fn reports_dynamic_expressions_without_executing_them() {
        let files = [(
            "justfile",
            "value := `touch SHOULD_NOT_EXIST`\nfetch:\n    #!/usr/bin/env nu\n    ^curl {{value}}\n\nstatic:\n    #!/usr/bin/env nu\n    ^curl https://example.com\n",
        )];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(
            stdout.contains("cannot statically render Just recipe"),
            "{stdout}"
        );
        assert!(stdout.contains("line 1:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
        assert!(!output.status.success());
    }

    #[test]
    fn does_not_substitute_parameter_defaults() {
        let files = [(
            "justfile",
            "URL := 'https://example.com'\nfetch URL='https://other.example.com':\n    #!/usr/bin/env nu\n    ^curl {{URL}}\n",
        )];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(
            stdout.contains("recipe parameter requires runtime arguments"),
            "{stdout}"
        );
        assert!(stdout.contains("line 4:"), "{stdout}");
        assert!(!output.status.success());
    }

    #[test]
    fn still_reports_missing_template_imports() {
        let files = [(
            "justfile",
            "fetch:\n    #!/usr/bin/env nu\n    source {{justfile_directory()}}/scripts/missing.nu\n",
        )];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(stdout.contains("File not found"), "{stdout}");
        assert!(stdout.contains("line 3:"), "{stdout}");
        assert!(!stdout.contains("Not a constant"), "{stdout}");
        assert!(!output.status.success());
    }

    #[test]
    fn reports_invalid_nushell_in_recipes() {
        let files = [("justfile", "fetch:\n    #!/usr/bin/env nu\n    let =\n")];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!output.status.success());
        assert!(stdout.contains("Error:"), "{stdout}");
        assert!(stdout.contains("justfile"), "{stdout}");
        assert!(stdout.contains("line 3:"), "{stdout}");
    }
}
