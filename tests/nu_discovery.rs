#[cfg(test)]
mod tests {
    use std::{fs, path, process, time};

    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LoggedAnalysis {
        timestamp: String,
        application: String,
        level: String,
        event: String,
        version: String,
        revision: String,
        project: path::PathBuf,
        language: String,
        file: path::PathBuf,
        recipe: Option<String>,
        category: String,
        reason: String,
    }

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
        let (output, _) = run_fixture_details(files, recipe)?;
        Ok(output)
    }

    fn run_logged_fixture(files: &[(&str, &str)]) -> (process::Output, String) {
        let sources: Vec<_> = files
            .iter()
            .map(|(path, source)| (*path, source.as_bytes()))
            .collect();
        let (output, log) = run_fixture_details(&sources, None).expect("fixture runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains("Warning:"), "{stdout}");
        assert!(!stdout.contains("skipped"), "{stdout}");
        assert!(output.stderr.is_empty(), "{:?}", output.stderr);
        let log = log.expect("skipped analyses are logged");
        let entries: Vec<LoggedAnalysis> = log
            .lines()
            .map(|line| serde_json::from_str(line).expect("log entry is structured JSON"))
            .collect();
        for entry in &entries {
            assert!(entry.timestamp.ends_with('Z'), "{entry:?}");
            assert_eq!(entry.application, "slop-detector");
            assert_eq!(entry.level, "ERROR");
            assert_eq!(entry.event, "analysis-failure");
            assert_eq!(entry.version, env!("CARGO_PKG_VERSION"));
            assert_eq!(entry.revision, env!("REVISION"));
            assert!(entry.project.is_absolute(), "{entry:?}");
            assert!(matches!(entry.language.as_str(), "nushell" | "just"));
            assert!(entry.file.is_relative(), "{entry:?}");
            assert!(matches!(
                entry.category.as_str(),
                "parse-error" | "unsupported-static-analysis"
            ));
            assert!(!entry.reason.is_empty(), "{entry:?}");
            if let Some(recipe) = &entry.recipe {
                assert!(!recipe.is_empty(), "{entry:?}");
            }
        }
        (output, log)
    }

    fn run_fixture_details(
        files: &[(&str, &[u8])],
        recipe: Option<&str>,
    ) -> color_eyre::Result<(process::Output, Option<String>)> {
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
        let home = root.join(".git/test-home");
        let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
            .arg("nu")
            .env("HOME", &home)
            .env_remove("XDG_STATE_HOME")
            .current_dir(&root)
            .output()?;
        let local_log = root.join(".slop-detector-skipped.log");
        if local_log.try_exists()? {
            return Err(color_eyre::eyre::eyre!(
                "project-local log must not be created"
            ));
        }
        let log_path = home.join(".local/state/slop-detector/errors.jsonl");
        let log = if log_path.try_exists()? {
            let contents = fs::read_to_string(log_path)?;
            Some(contents)
        } else {
            None
        };
        let marker = root.join("SHOULD_NOT_EXIST");
        let executed = marker.try_exists()?;
        fs::remove_dir_all(&root)?;
        if executed {
            return Err(color_eyre::eyre::eyre!("Just expressions must not execute"));
        }
        Ok((output, log))
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
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(log.contains("\"file\":\"recipes/fetch.just\""), "{log}");
        assert!(log.contains("\"recipe\":\"fetch\""), "{log}");
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
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(log.contains("cyclic Just variable"), "{log}");
        assert!(
            log.contains("\"category\":\"unsupported-static-analysis\""),
            "{log}"
        );
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn rejects_multiline_expansion_without_shifting_diagnostics() {
        let files = [(
            "justfile",
            "endpoint := \"https://example.com\\n\"\nfetch:\n    #!/usr/bin/env nu\n    ^curl {{endpoint}}\n",
        )];
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(
            log.contains("multiline interpolation is not supported"),
            "{log}"
        );
        assert!(log.contains("line 4"), "{log}");
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn reports_dynamic_expressions_without_executing_them() {
        let files = [(
            "justfile",
            "value := `touch SHOULD_NOT_EXIST`\nfetch:\n    #!/usr/bin/env nu\n    ^curl {{value}}\n\nstatic:\n    #!/usr/bin/env nu\n    ^curl https://example.com\n",
        )];
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(
            log.contains("expression requires runtime or unsupported evaluation"),
            "{log}"
        );
        assert!(log.contains("line 1"), "{log}");
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
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(
            log.contains("recipe parameter requires runtime arguments"),
            "{log}"
        );
        assert!(log.contains("line 4"), "{log}");
        assert!(log.contains("\"recipe\":\"fetch\""), "{log}");
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn still_reports_missing_template_imports() {
        let files = [(
            "justfile",
            "fetch:\n    #!/usr/bin/env nu\n    source {{justfile_directory()}}/scripts/missing.nu\n",
        )];
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(log.contains("\"file\":\"justfile\""), "{log}");
        assert!(log.contains("\"recipe\":\"fetch\""), "{log}");
        assert!(!stdout.contains("Not a constant"), "{stdout}");
        assert!(output.status.success(), "{stdout}");
    }

    #[test]
    fn does_not_log_clean_files_or_lint_findings() {
        let clean: &[u8] = b"print 'ready'\n";
        let violation: &[u8] = b"^curl https://example.com\n";
        for (source, success) in [(clean, true), (violation, false)] {
            let files = [("fixture.nu", source)];
            let (output, log) = run_fixture_details(&files, None).expect("fixture runs");
            assert_eq!(output.status.success(), success);
            assert!(log.is_none(), "{log:?}");
        }
    }

    #[test]
    fn appends_diagnostics_from_multiple_projects_to_one_home_log() {
        let timestamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("clock is after Unix epoch");
        let root = std::env::temp_dir().join(format!(
            "slop-detector-shared-log-{}-{}",
            process::id(),
            timestamp.as_nanos()
        ));
        let home = root.join("home");
        let state = root.join("state");
        let mut projects = Vec::new();
        for name in ["first", "second"] {
            let project = root.join(name);
            fs::create_dir_all(&project).expect("project directory is created");
            let source = project.join("broken.nu");
            fs::write(source, "let =\n").expect("fixture is written");
            let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
                .arg("nu")
                .env("HOME", &home)
                .env("XDG_STATE_HOME", &state)
                .current_dir(&project)
                .output()
                .expect("CLI runs");
            assert!(output.status.success());
            assert!(output.stderr.is_empty(), "{:?}", output.stderr);
            assert_eq!(output.stdout, "\u{2713} 0 files checked\n".as_bytes());
            projects.push(project);
        }
        assert!(!home.exists());
        let log_path = state.join("slop-detector/errors.jsonl");
        let log = fs::read_to_string(log_path).expect("shared log exists");
        let logged_projects: Vec<_> = log
            .lines()
            .map(|line| {
                let entry: LoggedAnalysis = serde_json::from_str(line).expect("JSON log entry");
                entry.project
            })
            .collect();
        assert_eq!(logged_projects, projects);
        fs::remove_dir_all(root).expect("fixture is removed");
    }

    #[test]
    fn logging_failure_preserves_cli_status() {
        let timestamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("clock is after Unix epoch");
        let root = std::env::temp_dir().join(format!(
            "slop-detector-unwritable-log-{}-{}",
            process::id(),
            timestamp.as_nanos()
        ));
        fs::create_dir_all(&root).expect("fixture directory exists");
        let state = root.join("blocked-state");
        fs::write(&state, "not a directory").expect("state path is blocked");
        let broken = root.join("broken.nu");
        fs::write(broken, "let =\n").expect("fixture exists");
        for (source, success) in [
            ("print 'ready'\n", true),
            ("^curl https://example.com\n", false),
        ] {
            let fixture = root.join("fixture.nu");
            fs::write(fixture, source).expect("fixture exists");
            let output = process::Command::new(env!("CARGO_BIN_EXE_slop-detector"))
                .arg("nu")
                .env("XDG_STATE_HOME", &state)
                .current_dir(&root)
                .output()
                .expect("CLI runs");
            let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
            assert_eq!(output.status.success(), success, "{stdout}");
            assert!(
                stdout.contains("Warning: could not write error log"),
                "{stdout}"
            );
            assert_eq!(stdout.contains("no-curl"), !success, "{stdout}");
        }
        fs::remove_dir_all(root).expect("fixture is removed");
    }

    #[test]
    fn logs_and_skips_invalid_nushell_files() {
        let files = [
            ("broken.nu", "let =\n"),
            ("scripts/broken", "#!/usr/bin/env nu\nlet =\n"),
            ("valid.nu", "http get https://example.com\n"),
        ];
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(output.status.success(), "{stdout}");
        assert!(log.contains("\"file\":\"broken.nu\""), "{log}");
        assert!(log.contains("\"file\":\"scripts/broken\""), "{log}");
        assert!(log.contains("\"category\":\"parse-error\""), "{log}");
        assert_eq!(log.lines().count(), 2, "{log}");
        assert_eq!(stdout, "\u{2713} 1 files checked\n");
    }

    #[test]
    fn still_fails_for_violations_in_other_files() {
        let files = [
            ("broken.nu", "use missing-module.nu\n"),
            ("fetch.nu", "^curl https://example.com\n"),
        ];
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!output.status.success());
        assert!(log.contains("\"file\":\"broken.nu\""), "{log}");
        assert!(!stdout.contains("Error:"), "{stdout}");
        assert_eq!(stdout.matches("no-curl").count(), 1, "{stdout}");
    }

    #[test]
    fn read_failures_remain_errors() {
        let files: &[(&str, &[u8])] = &[("broken.nu", b"\xff")];
        let (output, log) = run_fixture_details(files, None).expect("fixture runs");
        let log = log.expect("read failure is logged");
        assert!(log.contains("\"event\":\"operation-failure\""), "{log}");
        assert!(log.contains("\"operation\":\"read-file\""), "{log}");
        assert!(log.contains("\"path\":\"broken.nu\""), "{log}");
        assert!(log.contains("\"cause\":\"invalid data\""), "{log}");
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(!output.status.success());
        assert!(
            stdout.contains("Error: Failed to read ./broken.nu:"),
            "{stdout}"
        );
        assert!(!stdout.contains("Warning:"), "{stdout}");
    }

    #[test]
    fn shares_app_log_between_just_and_nushell() {
        let files = [
            (
                "justfile",
                "valid:\n    #!/usr/bin/env nu\n    print 'ready'\n\nbroken :=\n",
            ),
            ("broken.nu", "let =\n"),
        ];
        let (output, log) = run_logged_fixture(&files);
        assert!(output.status.success());
        let entries: Vec<LoggedAnalysis> = log
            .lines()
            .map(|line| serde_json::from_str(line).expect("JSON log entry"))
            .collect();
        assert_eq!(entries.len(), 2, "{log}");
        assert!(
            entries
                .iter()
                .any(|entry| entry.file == path::Path::new("justfile")
                    && entry.language == "just"
                    && entry.category == "parse-error"),
            "{log}"
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.file == path::Path::new("broken.nu")
                    && entry.language == "nushell"
                    && entry.category == "parse-error"),
            "{log}"
        );
        assert!(log.contains("line 5, column"), "{log}");
        assert!(!log.contains("invalid Just syntax"), "{log}");
        assert!(!log.contains("broken :="), "{log}");
        assert_eq!(output.stdout, "\u{2713} 0 files checked\n".as_bytes());
    }

    #[test]
    fn reports_invalid_nushell_in_recipes() {
        let files = [("justfile", "fetch:\n    #!/usr/bin/env nu\n    let =\n")];
        let (output, log) = run_logged_fixture(&files);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert!(output.status.success(), "{stdout}");
        assert!(log.contains("\"file\":\"justfile\""), "{log}");
        assert!(log.contains("\"recipe\":\"fetch\""), "{log}");
        assert!(log.contains("\"category\":\"parse-error\""), "{log}");
        assert!(log.contains("line 3, column "), "{log}");
        assert!(log.contains("nu::parser::"), "{log}");
        assert!(!log.contains("let ="), "{log}");
        assert_eq!(stdout, "\u{2713} 0 files checked\n");
    }
}
