#[cfg(test)]
mod tests {
    use std::{fs, process, time};

    fn run_fixture(files: &[(&str, &str)]) -> color_eyre::Result<process::Output> {
        let sources: Vec<_> = files
            .iter()
            .map(|(path, source)| (*path, source.as_bytes()))
            .collect();
        run_fixture_bytes(&sources)
    }

    fn run_fixture_bytes(files: &[(&str, &[u8])]) -> color_eyre::Result<process::Output> {
        run_fixture_with_oracle(files, None)
    }

    fn run_fixture_with_oracle(
        files: &[(&str, &[u8])],
        recipe: Option<&str>,
    ) -> color_eyre::Result<process::Output> {
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
        if let Some(name) = recipe {
            for (probe, relative) in [
                ("tasks::context-report", ""),
                ("tasks::override-report", "other"),
            ] {
                let actual = process::Command::new("just")
                    .arg(probe)
                    .current_dir(&root)
                    .output()?;
                let actual_directory = String::from_utf8(actual.stdout)?;
                let diagnostic = String::from_utf8(actual.stderr)?;
                let expected_path = root.join(relative);
                let expected_directory = fs::canonicalize(expected_path)?;
                if !actual.status.success()
                    || actual_directory.trim() != expected_directory.to_string_lossy()
                {
                    return Err(color_eyre::eyre::eyre!(
                        "Just working directory differs for {probe}: {actual_directory}; {diagnostic}"
                    ));
                }
            }
            let oracle = process::Command::new("just")
                .args(["--dry-run", name])
                .current_dir(&root)
                .output()?;
            let rendered = String::from_utf8(oracle.stderr)?;
            if !oracle.status.success() {
                return Err(color_eyre::eyre::eyre!("Just oracle failed: {rendered}"));
            }
            for relative in [
                "root-env.nu",
                ".just/module-env.nu",
                "recipes/source-env.nu",
            ] {
                let absolute = root.join(relative);
                let expected = absolute.to_string_lossy();
                let expected_path = expected.as_ref();
                if !rendered.contains(expected_path) {
                    return Err(color_eyre::eyre::eyre!(
                        "Just oracle did not render {expected}: {rendered}"
                    ));
                }
            }
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
    fn allows_explicit_nonzero_exit_code_handling_before_rethrowing() {
        let codes: [i64; 3] = [1, 2, 130];
        for code in codes {
            let source = format!(
                "def main [] {{\nlet result = (^fzf | complete)\nif $result.exit_code == {code} {{ return }}\nif $result.exit_code != 0 {{ error make {{msg: $result.stderr}} }}\n$result.stdout\n}}\n"
            );
            let output = run_fixture(&[("picker.nu", &source)]).expect("fixture runs");
            let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
            assert!(!stdout.contains("no-complete-rethrow"), "{stdout}");
            assert!(!stdout.contains("Warning:"), "{stdout}");
            assert_eq!(stdout.matches("no-error-make").count(), 1, "{stdout}");
        }
    }

    #[test]
    fn still_flags_rethrows_without_a_prior_exception_for_the_same_result() {
        for handler in [
            "",
            "if $result.exit_code == 0 { print 'success' }",
            "if $other.exit_code == 130 { return }",
            "if true { if $result.exit_code == 130 { return } }",
        ] {
            let source = format!(
                "def main [] {{\nlet result = (^fzf | complete)\nlet other = (^git status | complete)\n{handler}\nif $result.exit_code != 0 {{ error make {{msg: $result.stderr}} }}\n}}\n"
            );
            let output = run_fixture(&[("picker.nu", &source)]).expect("fixture runs");
            let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
            assert!(!stdout.contains("Warning:"), "{stdout}");
            assert_eq!(stdout.matches("no-complete-rethrow").count(), 1, "{stdout}");
        }
    }

    #[test]
    fn still_flags_rethrows_in_else_if_branches() {
        let source = "let result = (^fzf | complete)\nif $result.exit_code == 0 { print $result.stdout } else if $result.exit_code != 0 { error make {msg: $result.stderr} }\n";
        let output = run_fixture(&[("picker.nu", source)]).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Warning:"), "{stdout}");
        assert_eq!(stdout.matches("no-complete-rethrow").count(), 1, "{stdout}");
    }

    #[test]
    fn later_exit_code_handling_does_not_exempt_an_earlier_rethrow() {
        let source = "let result = (^fzf | complete)\nif $result.exit_code != 0 { error make {msg: $result.stderr} }\nif $result.exit_code == 130 { return }\n";
        let output = run_fixture(&[("picker.nu", source)]).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Warning:"), "{stdout}");
        assert_eq!(stdout.matches("no-complete-rethrow").count(), 1, "{stdout}");
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
    fn matches_just_root_module_and_source_contexts() {
        let files: &[(&str, &[u8])] = &[
            ("justfile", b"mod tasks '.just/tasks.just'\n"),
            (".just/tasks.just", b"set working-directory := '..'\nimport '../recipes/fetch.just'\n"),
            ("root-env.nu", b"let root_url = 'https://root.example.com'\n"),
            (".just/module-env.nu", b"let module_url = 'https://module.example.com'\n"),
            ("recipes/source-env.nu", b"let source_url = 'https://source.example.com'\n"),
            ("recipes/fetch.just", b"fetch:\n    #!/usr/bin/env nu\n    source '{{justfile_directory()}}/root-env.nu'\n    source '{{module_directory()}}/module-env.nu'\n    source '{{source_directory()}}/source-env.nu'\n    ^curl $root_url $module_url $source_url\n\ncontext-report:\n    #!/usr/bin/env nu\n    print $env.PWD\n\n[working-directory: 'other']\noverride-report:\n    #!/usr/bin/env nu\n    print $env.PWD\n"),
            ("other/env.nu", b"let endpoint = 'https://other.example.com'\n"),
        ];
        let recipe = Some("tasks::fetch");
        let output = run_fixture_with_oracle(files, recipe).expect("fixture matches Just");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Warning:"), "{stdout}");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
    }

    #[test]
    fn does_not_resolve_relative_imports_from_the_project_directory() {
        let files = [
            ("justfile", "mod tasks '.just/tasks.just'\n"),
            (".just/tasks.just", "import '../recipes/fetch.just'\n"),
            (".just/env.nu", "let endpoint = 'https://example.com'\n"),
            (
                "recipes/fetch.just",
                "fetch:\n    #!/usr/bin/env nu\n    source env.nu\n    ^curl $endpoint\n",
            ),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(stdout.contains("Warning: [nushell] skipped recipe fetch from ./recipes/fetch.just: import could not be resolved statically"), "{stdout}");
        assert!(!stdout.contains("no-curl"), "{stdout}");
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn respects_working_directory_settings_and_recipe_overrides() {
        let files = [
            (
                "justfile",
                "set working-directory := 'scripts'\nfetch:\n    #!/usr/bin/env nu\n    source '{{justfile_directory()}}/scripts/env.nu'\n    ^curl $endpoint\n\n[working-directory: 'other']\nfetch-other:\n    #!/usr/bin/env nu\n    source '{{justfile_directory()}}/scripts/other/env.nu'\n    ^curl $endpoint\n",
            ),
            ("scripts/env.nu", "let endpoint = 'https://example.com'\n"),
            (
                "scripts/other/env.nu",
                "let endpoint = 'https://other.example.com'\n",
            ),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Warning:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 2, "{stdout}");
    }

    #[test]
    fn applies_imported_working_directory_settings_to_the_module() {
        let files = [
            (
                "justfile",
                "import 'config/settings.just'\nfetch:\n    #!/usr/bin/env nu\n    source '{{justfile_directory()}}/scripts/env.nu'\n    ^curl $endpoint\n",
            ),
            (
                "config/settings.just",
                "set working-directory := 'scripts'\n",
            ),
            ("scripts/env.nu", "let endpoint = 'https://example.com'\n"),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!stdout.contains("Warning:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
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
        assert!(
            stdout.contains("Warning: [nushell] skipped recipe fetch from ./justfile:"),
            "{stdout}"
        );
        assert!(output.status.success(), "{stdout}");
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
        assert!(stdout.contains("line 4"), "{stdout}");
        assert!(
            stdout.contains("Warning: [nushell] skipped recipe fetch from ./justfile:"),
            "{stdout}"
        );
        assert!(output.status.success(), "{stdout}");
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
            stdout.contains("expression requires runtime or unsupported evaluation"),
            "{stdout}"
        );
        assert!(stdout.contains("line 1"), "{stdout}");
        assert!(
            stdout.contains("Warning: [nushell] skipped recipe fetch from ./justfile:"),
            "{stdout}"
        );
        assert!(!stdout.contains("Error:"), "{stdout}");
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
        assert!(stdout.contains("line 4"), "{stdout}");
        assert!(
            stdout.contains("Warning: [nushell] skipped recipe fetch from ./justfile:"),
            "{stdout}"
        );
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn still_reports_missing_template_imports() {
        let files = [(
            "justfile",
            "fetch:\n    #!/usr/bin/env nu\n    source {{justfile_directory()}}/scripts/missing.nu\n",
        )];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(stdout.contains("Warning: [nushell] skipped recipe fetch from ./justfile: import could not be resolved statically"), "{stdout}");
        assert!(!stdout.contains("Not a constant"), "{stdout}");
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn warns_and_skips_invalid_nushell_files() {
        let files = [
            ("broken.nu", "let =\n"),
            ("scripts/broken", "#!/usr/bin/env nu\nlet =\n"),
            ("valid.nu", "http get https://example.com\n"),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(output.status.success(), "{stdout}");
        assert!(
            stdout.contains("Warning: [nushell] skipped file ./broken.nu:"),
            "{stdout}"
        );
        assert!(
            stdout.contains("Warning: [nushell] skipped file ./scripts/broken:"),
            "{stdout}"
        );
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert!(
            stdout.contains("Checked files: 1; skipped analyses: 2"),
            "{stdout}"
        );
    }

    #[test]
    fn still_fails_for_violations_in_other_files() {
        let files = [
            ("broken.nu", "use missing-module.nu\n"),
            ("fetch.nu", "^curl https://example.com\n"),
        ];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!output.status.success());
        assert!(
            stdout.contains("Warning: [nushell] skipped file ./broken.nu:"),
            "{stdout}"
        );
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
    }

    #[test]
    fn read_failures_remain_errors() {
        let files: &[(&str, &[u8])] = &[("broken.nu", b"\xff")];
        let output = run_fixture_bytes(files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!output.status.success());
        assert!(
            stdout.contains("Error: Failed to read ./broken.nu:"),
            "{stdout}"
        );
        assert!(!stdout.contains("Warning:"), "{stdout}");
    }

    #[test]
    fn reports_invalid_nushell_in_recipes() {
        let files = [("justfile", "fetch:\n    #!/usr/bin/env nu\n    let =\n")];
        let output = run_fixture(&files).expect("fixture runs");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(output.status.success(), "{stdout}");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert!(
            stdout.contains("Warning: [nushell] skipped recipe fetch from ./justfile:"),
            "{stdout}"
        );
        assert!(stdout.contains("line 3:"), "{stdout}");
        assert!(
            stdout.contains("Checked files: 0; skipped analyses: 1"),
            "{stdout}"
        );
    }
}
