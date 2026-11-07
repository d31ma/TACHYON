use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tachyon_contracts::CONTRACTS;
use yaml_rust2::{Yaml, YamlLoader};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            Path::to_path_buf,
        )
}

fn collect_files(root: &Path, extension: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn relative_markdown_links(contents: &str) -> Vec<&str> {
    let mut links = Vec::new();
    let mut fence = None;
    for line in contents.lines() {
        if fenced_code_line(line, &mut fence) {
            continue;
        }
        collect_line_links(line, &mut links);
    }
    links
}

fn fenced_code_line(line: &str, fence: &mut Option<(char, usize)>) -> bool {
    let text = line.trim_start_matches(' ');
    if line.len() - text.len() > 3 || text.starts_with('\t') {
        return fence.is_some();
    }
    let marker = text.chars().next().unwrap_or(' ');
    let length = text.chars().take_while(|value| *value == marker).count();
    let is_marker = matches!(marker, '`' | '~') && length >= 3;
    if let Some((opened, minimum)) = *fence {
        if is_marker && marker == opened && length >= minimum && text[length..].trim().is_empty() {
            *fence = None;
        }
        return true;
    }
    if !is_marker || (marker == '`' && text[length..].contains('`')) {
        return false;
    }
    *fence = Some((marker, length));
    true
}

fn collect_line_links<'a>(line: &'a str, links: &mut Vec<&'a str>) {
    for tail in line.split("](").skip(1) {
        let Some(target) = tail.split(')').next() else {
            continue;
        };
        let target = target.trim();
        if target.is_empty()
            || target.starts_with('#')
            || target.contains("://")
            || target.starts_with("mailto:")
        {
            continue;
        }
        links.push(target);
    }
}

#[test]
fn document_links_ignore_fenced_code_without_hiding_real_links() {
    let source = "[before](before.md)\n```go\nBehaviorsProperties[CatalogSettings](application)\n```\n[after](after.md)\n~~~~text\n[example](not-a-link)\n~~~\n[still code](not-a-link-either)\n~~~~\n[final](final.md)\n";
    assert_eq!(
        relative_markdown_links(source),
        ["before.md", "after.md", "final.md"]
    );
}

#[test]
fn document_links_accept_up_to_three_spaces_before_fences() {
    for width in 0..=3 {
        let indent = " ".repeat(width);
        for marker in ["```", "~~~"] {
            let source = format!(
                "[before](before.md)\n{indent}{marker}go\n[code](ignored)\n{indent}{marker}\n[after](after.md)\n"
            );
            assert_eq!(relative_markdown_links(&source), ["before.md", "after.md"]);
        }
    }
}

#[test]
fn document_links_do_not_open_indented_or_invalid_backtick_fences() {
    for opening in ["    ```go", "\t```go", "   \t```go", "```go`value"] {
        let source = format!("{opening}\n\n[real](real.md)\n");
        assert_eq!(relative_markdown_links(&source), ["real.md"], "{opening:?}");
    }
}

#[test]
fn document_links_do_not_close_on_indented_fences() {
    for indent in ["    ", "\t", "   \t"] {
        let source = format!("```go\n{indent}```\n[still code](ignored)\n```\n[after](after.md)\n");
        assert_eq!(relative_markdown_links(&source), ["after.md"], "{indent:?}");
    }
}

#[test]
fn document_links_allow_backticks_in_tilde_fence_info() {
    let source = "~~~`go`\n[code](ignored)\n~~~\n[after](after.md)\n";
    assert_eq!(relative_markdown_links(source), ["after.md"]);
}

#[test]
fn required_foundation_documents_are_present_and_substantive()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let required = [
        "CLAUDE.md",
        "CHANGELOG.md",
        "CODE_OF_CONDUCT.md",
        "CONTEXT.md",
        "CONTRIBUTING.md",
        "GOVERNANCE.md",
        "README.md",
        "SECURITY.md",
        "SUPPORT.md",
        "docs/ENGINEERING_STANDARDS.md",
        "docs/PHASE_1_EVIDENCE.md",
        "docs/PHASE_1_SPEC.md",
        "docs/PHASE_2_EVIDENCE.md",
        "docs/PHASE_2_SPEC.md",
        "docs/PHASE_3_EVIDENCE.md",
        "docs/PHASE_3_IMPLEMENTATION_PLAN.md",
        "docs/PHASE_3_SPEC.md",
        "docs/PHASE_4_EVIDENCE.md",
        "docs/PHASE_4_IMPLEMENTATION_PLAN.md",
        "docs/PHASE_4_SPEC.md",
        "docs/PROJECT_PLAN.md",
        "docs/RELEASE_ENGINEERING.md",
        "docs/SUPPORT_TIERS.md",
        "docs/THREAT_MODEL.md",
        "docs/architecture/OVERVIEW.md",
    ];

    for relative in required {
        let contents = fs::read_to_string(root.join(relative))?;
        assert!(
            contents.lines().count() >= 10,
            "{relative} is missing or not substantive"
        );
    }

    Ok(())
}

#[test]
fn a_local_gate_builds_the_release_binary_with_the_shipped_features()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let shipped = "build --release --locked -p tachyon-cli --bin ty --features storage-all";
    let artifact = fs::read_to_string(root.join("scripts/release/build-artifact.sh"))?;
    let package = fs::read_to_string(root.join("package.json"))?;
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    assert!(
        artifact.contains(&format!("({shipped})")),
        "build-artifact.sh changed what it ships"
    );
    assert!(
        package.contains(&format!("\"test:release-build\": \"cargo {shipped}\"")),
        "package.json has no test:release-build gate for the shipped build"
    );
    assert!(
        workflow.contains(&format!("cargo auditable {shipped}")),
        "rust-ci.yml builds something else than the shipped binary"
    );
    Ok(())
}

#[test]
fn release_artifact_inputs_are_present_and_nonempty() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    for relative in ["LICENSE", "NOTICE", "VERSION", "docs/SUPPORT_TIERS.md"] {
        let contents = fs::read(root.join(relative))?;
        assert!(!contents.is_empty(), "{relative} is empty");
    }
    Ok(())
}

#[test]
fn every_phase7_fuzz_boundary_has_a_target_seed_and_ci_campaign()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let manifest = fs::read_to_string(root.join("fuzz/Cargo.toml"))?;
    let lockfile = fs::read_to_string(root.join("fuzz/Cargo.lock"))?;
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    let scheduled = fs::read_to_string(root.join(".github/workflows/fuzz-scheduled.yml"))?;

    assert!(manifest.contains("libfuzzer-sys = \"=0.4.10\""));
    assert!(lockfile.contains("name = \"libfuzzer-sys\""));
    for target in [
        "template_frontend",
        "handler_frame",
        "native_planner",
        "yon_annotations",
        "application_config",
        "managed_resource_headers",
    ] {
        assert!(manifest.contains(&format!("name = \"{target}\"")));
        assert!(
            root.join(format!("fuzz/fuzz_targets/{target}.rs"))
                .is_file()
        );
        let seeds = fs::read_dir(root.join(format!("fuzz/corpus/{target}")))?
            .collect::<Result<Vec<_>, _>>()?;
        assert!(!seeds.is_empty(), "{target} has no hand-written seed");
    }
    assert!(workflow.contains(
        "for target in template_frontend handler_frame native_planner yon_annotations application_config managed_resource_headers"
    ));
    assert!(workflow.contains("-max_total_time=${FUZZ_SECONDS}"));
    assert!(scheduled.contains("name: Fuzz ${{ matrix.target }}"));
    assert!(scheduled.contains("target: [template_frontend, handler_frame, native_planner, yon_annotations, application_config, managed_resource_headers]"));
    assert!(scheduled.contains("path: fuzz/corpus/${{ matrix.target }}"));
    assert!(scheduled.contains("actions/cache/restore@caa296126883cff596d87d8935842f9db880ef25"));
    assert!(scheduled.contains("actions/cache/save@caa296126883cff596d87d8935842f9db880ef25"));
    assert!(
        scheduled
            .contains("fuzz-corpus-${{ runner.os }}-${{ matrix.target }}-${{ github.run_id }}")
    );
    Ok(())
}

#[test]
fn architecture_adrs_have_final_statuses_and_are_numbered() -> Result<(), Box<dyn std::error::Error>>
{
    let root = repository_root();
    let adrs = collect_files(&root.join("docs/adr"), "md")?;
    let numbered = adrs
        .iter()
        .filter(|path| is_numbered_adr(path))
        .collect::<Vec<_>>();

    assert!(!numbered.is_empty());
    for (index, adr) in numbered.into_iter().enumerate() {
        let expected_prefix = format!("{:04}-", index + 1);
        assert!(
            adr.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with(&expected_prefix)),
            "{} is not the expected ADR {expected_prefix}",
            adr.display()
        );
        let contents = fs::read_to_string(adr)?;
        assert!(
            contents
                .lines()
                .take(12)
                .any(|line| line.contains("Accepted") || line.contains("Superseded")),
            "{} has no final status",
            adr.display()
        );
    }

    Ok(())
}

fn is_numbered_adr(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let bytes = name.as_bytes();
    bytes.len() > 5 && bytes[..4].iter().all(u8::is_ascii_digit) && bytes[4] == b'-'
}

#[test]
fn schema_directories_and_registry_cannot_drift() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let schemas = collect_files(&root.join("api"), "json")?
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("schema.json"))
        .collect::<Vec<_>>();

    let registered = CONTRACTS
        .iter()
        .map(|contract| contract.name)
        .collect::<BTreeSet<_>>();
    let on_disk = schemas
        .iter()
        .filter_map(|path| {
            path.ancestors()
                .nth(2)
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(on_disk, registered);
    Ok(())
}

#[test]
fn github_actions_are_pinned_to_full_commits() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflows = collect_files(&root.join(".github/workflows"), "yml")?;

    for workflow in workflows {
        let contents = fs::read_to_string(&workflow)?;
        assert_pinned_actions(&root, &workflow, &contents);
    }
    Ok(())
}

#[test]
fn the_workflow_folder_holds_exactly_the_parsed_workflows() -> Result<(), Box<dyn std::error::Error>>
{
    let expected = [
        "fuzz-scheduled.yml",
        "publish.yml",
        "ruby-runtime.yml",
        "rust-ci.yml",
    ]
    .map(String::from);
    assert_eq!(workflow_file_names()?, BTreeSet::from(expected));
    Ok(())
}

#[test]
fn the_rust_ci_workflow_parses_as_yaml_with_jobs() -> Result<(), Box<dyn std::error::Error>> {
    parsed_workflow("rust-ci.yml")?;
    Ok(())
}

#[test]
fn the_publish_workflow_parses_as_yaml_with_jobs() -> Result<(), Box<dyn std::error::Error>> {
    parsed_workflow("publish.yml")?;
    Ok(())
}

#[test]
fn the_ruby_runtime_workflow_parses_as_yaml_with_jobs() -> Result<(), Box<dyn std::error::Error>> {
    parsed_workflow("ruby-runtime.yml")?;
    Ok(())
}

#[test]
fn the_fuzz_workflow_parses_as_yaml_with_jobs() -> Result<(), Box<dyn std::error::Error>> {
    parsed_workflow("fuzz-scheduled.yml")?;
    Ok(())
}

#[test]
fn the_only_token_minting_job_refuses_pull_requests_from_forks()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let contents = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    let workflow = parsed_workflow("rust-ci.yml")?;
    let job = &workflow["jobs"]["release-lifecycle"];
    assert_eq!(contents.matches("id-token:").count(), 1);
    assert!(!contents.contains("write-all"));
    assert_eq!(job["permissions"]["id-token"].as_str(), Some("write"));
    assert_eq!(
        job["if"].as_str(),
        Some(
            "(github.event_name != 'pull_request' || ((github.head_ref == 'develop' || startsWith(github.head_ref, 'release/')) && github.event.pull_request.head.repo.full_name == github.repository)) && (!inputs.jobs || contains(format(',{0},', inputs.jobs), ',release-lifecycle,'))"
        )
    );
    Ok(())
}

#[test]
fn every_job_testing_tachyon_core_installs_typescript_first()
-> Result<(), Box<dyn std::error::Error>> {
    let workflow = parsed_workflow("rust-ci.yml")?;
    let jobs = workflow["jobs"].as_hash().ok_or("rust-ci.yml has jobs")?;
    let mut checked = 0;
    for (name, job) in jobs {
        let runs = step_commands(job);
        let Some(first_test) = runs.iter().position(|run| tests_tachyon_core(run)) else {
            continue;
        };
        checked += 1;
        assert!(
            runs[..first_test]
                .iter()
                .any(|run| installs_typescript(run)),
            "job {name:?} tests tachyon-core before it installs the pinned TypeScript compiler"
        );
    }
    assert!(
        checked >= 4,
        "the native, macos-native, sanitizers and coverage jobs test tachyon-core"
    );
    Ok(())
}

fn step_commands(job: &Yaml) -> Vec<&str> {
    job["steps"]
        .as_vec()
        .map(|steps| {
            steps
                .iter()
                .filter_map(|step| step["run"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

fn tests_tachyon_core(run: &str) -> bool {
    let tests = [" test ", "nextest", "llvm-cov"]
        .iter()
        .any(|command| run.contains(command));
    let covers_core = run.contains("-p tachyon-core") || run.contains("--workspace");
    run.contains("cargo") && tests && covers_core
}

fn installs_typescript(run: &str) -> bool {
    run.contains("npm install --no-save typescript@")
        || run.contains("bun install --frozen-lockfile")
}

fn parsed_workflow(name: &str) -> Result<Yaml, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(repository_root().join(".github/workflows").join(name))?;
    let mut documents = YamlLoader::load_from_str(&contents)
        .map_err(|error| format!("{name} is not valid YAML: {error}"))?;
    assert_eq!(documents.len(), 1, "{name} holds one YAML document");
    let workflow = documents.remove(0);
    assert!(
        workflow["jobs"]
            .as_hash()
            .is_some_and(|jobs| !jobs.is_empty()),
        "{name} declares its jobs as a mapping"
    );
    Ok(workflow)
}

fn workflow_file_names() -> Result<BTreeSet<String>, std::io::Error> {
    let directory = repository_root().join(".github/workflows");
    let mut files = collect_files(&directory, "yml")?;
    files.extend(collect_files(&directory, "yaml")?);
    Ok(files
        .iter()
        .filter_map(|file| file.file_name()?.to_str().map(str::to_owned))
        .collect())
}

#[test]
fn workflow_parse_check_rejects_a_plain_scalar_ending_in_a_colon() {
    let workflow = "jobs:\n  native:\n    steps:\n      - run: cargo nextest run storage::\n";
    assert!(YamlLoader::load_from_str(workflow).is_err());
}

fn assert_pinned_actions(root: &Path, workflow: &Path, contents: &str) {
    for line in contents.lines() {
        let Some(action) = line.trim().strip_prefix("uses: ") else {
            continue;
        };
        if let Some(local) = action
            .split_whitespace()
            .next()
            .filter(|value| value.starts_with("./"))
        {
            assert!(
                root.join(local.trim_start_matches("./")).exists(),
                "{} references a missing local workflow: {action}",
                workflow.display()
            );
            continue;
        }
        let revision = action
            .split('#')
            .next()
            .and_then(|value| value.rsplit_once('@'))
            .map(|(_, revision)| revision.trim())
            .unwrap_or_default();
        assert!(
            revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{} contains an unpinned action: {action}",
            workflow.display()
        );
    }
}

#[test]
fn project_environment_variables_use_domain_namespaces() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let mut files = Vec::new();
    for directory in [".github", "crates", "scripts", "website/tests"] {
        for extension in ["js", "mjs", "ps1", "rs", "sh", "ts", "yaml", "yml"] {
            files.extend(collect_files(&root.join(directory), extension)?);
        }
    }

    let old_namespace = ["TACH", "YON_"].concat();
    let removed_data_namespace = ["FY", "LO_"].concat();
    let removed_packaging_name = ["YON", "_DIST_PATH"].concat();
    let forbidden = [
        format!("process.env.{old_namespace}"),
        format!("std::env::var(\"{old_namespace}"),
        format!("std::env::var_os(\"{old_namespace}"),
        format!(".env(\"{old_namespace}"),
        format!(".env_remove(\"{old_namespace}"),
        format!("env!(\"{old_namespace}"),
        format!("cargo:rustc-env={old_namespace}"),
        format!("$env:{old_namespace}"),
        format!("${{{old_namespace}"),
    ];
    let mut violations = Vec::new();
    for file in files {
        if file
            .components()
            .any(|component| component.as_os_str() == "fixtures")
        {
            continue;
        }
        let contents = fs::read_to_string(&file)?;
        for (index, line) in contents.lines().enumerate() {
            let workflow_environment = line
                .trim_start()
                .strip_prefix(&old_namespace)
                .is_some_and(|value| value.contains(':'));
            if workflow_environment
                || forbidden.iter().any(|pattern| line.contains(pattern))
                || line.contains(&removed_data_namespace)
                || line.contains(&removed_packaging_name)
            {
                violations.push(format!("{}:{}", file.display(), index + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "project environment variables must start with TAC_ or YON_, and removed namespaces must not return: {violations:?}"
    );
    Ok(())
}

#[test]
fn legacy_framework_tree_cannot_return() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    for relative in [
        "src",
        "tests",
        "scripts/bench",
        "scripts/install-vendor-bins.sh",
        "scripts/typecheck.js",
        "scripts/windows-native-compile-smoke.js",
        "bunfig.toml",
        "tsconfig.json",
        "tsconfig.base.json",
        "tsconfig.src.json",
        "tsconfig.tests.json",
        "tsconfig.website.json",
    ] {
        assert!(
            !root.join(relative).exists(),
            "legacy implementation path returned: {relative}"
        );
    }

    let package = fs::read_to_string(root.join("package.json"))?;
    for legacy_command in ["src/cli/", "bun test ./tests", "tests/.env.test"] {
        assert!(
            !package.contains(legacy_command),
            "legacy package command returned: {legacy_command}"
        );
    }
    assert!(root.join("types/tachyon-env.d.ts").is_file());
    assert!(
        root.join("crates/tachyon-cli/tests/fixtures/migration-fullstack")
            .is_dir()
    );

    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    assert!(workflow.contains("Rewrite-only repository policy"));
    assert!(workflow.contains("TAC_LEGACY_BIN="));
    assert!(workflow.contains("crates/tachyon-cli/tests/fixtures/migration-fullstack"));
    Ok(())
}

#[test]
fn the_clients_folder_is_named_only_where_it_stays_an_alias()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let allowed = [
        "crates/tachyon-core/src/annotation_layout.rs",
        "crates/tachyon-core/src/migrate.rs",
        "crates/tachyon-core/src/migrate/fix/rename_tests.rs",
        "crates/tachyon-core/src/migrate/fix/route_tests.rs",
        "crates/tachyon-core/src/project.rs",
        "crates/tachyon-core/src/project/captured_sources.rs",
        "crates/tachyon-core/src/template/service.rs",
    ];
    let mut files = Vec::new();
    for folder in [
        "crates/tachyon-core/src",
        "crates/tachyon-core/tests/it",
        "crates/tachyon-cli/src",
        "crates/tachyon-cli/tests",
    ] {
        files.extend(collect_files(&root.join(folder), "rs")?);
    }
    files.extend(collect_files(&root.join("scripts"), "mjs")?);
    let mut stale = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)?
            .to_string_lossy()
            .replace('\\', "/");
        let contents = fs::read_to_string(&file)?;
        let names_old_folder =
            contents.contains("client/clients") || contents.contains("server/clients");
        if names_old_folder && !allowed.contains(&relative.as_str()) {
            stale.push(relative);
        }
    }
    assert!(
        stale.is_empty(),
        "outbound clients live in client/remotes or server/remotes (ADR 0088); clients/ stays only as an alias: {stale:?}"
    );
    Ok(())
}

#[test]
fn stable_release_is_tag_gated_and_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/publish.yml"))?;
    let unix_installer = fs::read_to_string(root.join("install.sh"))?;
    let windows_installer = fs::read_to_string(root.join("install.ps1"))?;

    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(!workflow.contains("branches: [main]"));
    assert!(workflow.contains("RELEASE_TAG: ${{ inputs.tag || github.ref_name }}"));
    assert!(workflow.contains("DEFAULT_BRANCH: ${{ github.event.repository.default_branch }}"));
    assert!(workflow.matches("contents: write").count() >= 4);
    assert!(workflow.contains("git cat-file -t"));
    assert!(workflow.contains("uses: ./.github/workflows/rust-ci.yml"));
    assert!(workflow.contains("--verify-tag --draft"));
    assert!(workflow.contains("gh attestation verify"));
    assert!(workflow.contains("cosign verify-blob"));
    assert!(workflow.contains("sha256sum --check --strict SHA256SUMS"));
    assert!(workflow.contains("release-bad"));
    assert!(workflow.contains("Copy-Item 'release/SHA256SUMS' 'release-bad/SHA256SUMS'"));
    assert!(workflow.contains("[IO.File]::AppendAllText"));
    assert!(workflow.contains("Checksum mismatch for $env:ASSET. Aborting."));
    assert!(
        workflow.contains("gh release download \"${TAG}\" --repo \"${REPOSITORY}\" --dir release")
    );
    assert!(
        workflow
            .contains("gh release edit \"${TAG}\" --repo \"${REPOSITORY}\" --draft=false --prerelease --latest=false")
    );
    assert!(workflow.contains("failed upgrade replaced the installed executable"));
    assert!(workflow.contains("./scripts/release/lifecycle-drill.sh --candidate"));
    assert!(!workflow.contains("environment: packages-prod"));
    assert!(workflow.contains("needs.verify-release.result == 'success'"));

    assert!(unix_installer.contains("No checksum published for ${asset}. Aborting."));
    assert!(unix_installer.contains("mktemp \"${dest}/.ty.download.XXXXXX\""));
    assert!(unix_installer.contains("chmod 0755 \"$tmp\""));
    assert!(unix_installer.contains("mv \"$tmp\" \"$dest/ty\""));
    assert!(!unix_installer.contains("mktemp -d"));
    assert!(windows_installer.contains("No checksum published for $asset. Aborting."));
    assert!(windows_installer.contains("[Text.Encoding]::UTF8.GetString($sumsContent)"));
    assert!(!windows_installer.contains("Could not verify checksum"));
    assert!(windows_installer.contains("[IO.File]::Replace($download, $exe, $backup)"));
    assert!(!windows_installer.contains("-OutFile $exe"));
    assert!(!unix_installer.contains("d31ma/Fylo"));
    assert!(!windows_installer.contains("d31ma/Fylo"));
    assert!(!unix_installer.contains("releases/latest/download/install.sh"));
    assert!(!windows_installer.contains("releases/latest/download/install.ps1"));
    Ok(())
}

#[test]
fn release_signatures_and_downloads_use_the_current_repository_name()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/publish.yml"))?;
    let unix_installer = fs::read_to_string(root.join("install.sh"))?;
    let windows_installer = fs::read_to_string(root.join("install.ps1"))?;
    let manifest = fs::read_to_string(root.join("Cargo.toml"))?;

    assert!(workflow.contains(
        "--certificate-identity-regexp '^https://github.com/d31ma/TACHYON/.github/workflows/publish.yml@refs/tags/v'"
    ));
    assert!(!workflow.contains("d31ma/Tachyon"));
    assert!(unix_installer.contains("REPO=\"d31ma/TACHYON\""));
    assert!(windows_installer.contains("$repo = 'd31ma/TACHYON'"));
    assert!(manifest.contains("repository = \"https://github.com/d31ma/TACHYON\""));
    Ok(())
}

#[test]
fn the_snippet_job_builds_ty_with_every_storage_driver() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    let release_build = fs::read_to_string(root.join("scripts/release/build-artifact.sh"))?;

    assert!(workflow.contains("run: cargo build --locked --bin ty --features storage-all"));
    assert!(workflow.contains("run: node website/scripts/verify-snippets.mjs"));
    assert!(workflow.contains("TAC_BIN: ${{ github.workspace }}/target/debug/ty"));
    assert!(release_build.contains("--bin ty --features storage-all"));
    Ok(())
}

#[test]
fn foundation_document_links_resolve_locally() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let mut documents = collect_files(&root.join("docs"), "md")?;
    documents.push(root.join("CLAUDE.md"));
    documents.push(root.join("CONTEXT.md"));
    documents.push(root.join("CONTRIBUTING.md"));
    documents.push(root.join("GOVERNANCE.md"));
    documents.push(root.join("SECURITY.md"));
    documents.push(root.join("SUPPORT.md"));
    documents.push(root.join("api/README.md"));

    for document in documents {
        let contents = fs::read_to_string(&document)?;
        let parent = document.parent().unwrap_or(&root);
        for target in relative_markdown_links(&contents) {
            let path = target.split('#').next().unwrap_or_default();
            assert!(
                parent.join(path).exists(),
                "{} links to missing path {target}",
                document.display()
            );
        }
    }

    Ok(())
}

#[test]
fn exact_toolchain_is_consistent_across_repository_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let toolchain = fs::read_to_string(root.join("rust-toolchain.toml"))?;
    let workspace = fs::read_to_string(root.join("Cargo.toml"))?;
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;

    assert!(toolchain.contains("channel = \"1.97.1\""));
    assert!(workspace.contains("rust-version = \"1.97.1\""));
    assert!(workflow.contains("RUST_TOOLCHAIN: \"1.97.1\""));
    assert!(workflow.contains("rustup toolchain install 1.97.1"));
    Ok(())
}

#[test]
fn github_ci_runs_only_for_develop_or_release_into_main() -> Result<(), Box<dyn std::error::Error>>
{
    let root = repository_root();
    let guard = "if: github.event_name != 'pull_request' || github.head_ref == 'develop' || startsWith(github.head_ref, 'release/')";
    for name in ["rust-ci.yml", "ruby-runtime.yml"] {
        let workflow = fs::read_to_string(root.join(".github/workflows").join(name))?;
        assert!(
            workflow.contains("pull_request:\n    branches: [main]"),
            "{name}"
        );
        assert!(
            !workflow.contains("\n  push:"),
            "{name} must not run on pushes"
        );
        let jobs = workflow.split("\njobs:\n").nth(1).unwrap_or_default();
        let lines: Vec<&str> = jobs.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let is_job = line.len() > 3
                && line.starts_with("  ")
                && !line.starts_with("   ")
                && line.ends_with(':');
            if is_job {
                let next = lines.get(index + 1).map(|next| next.trim());
                let job = line.trim().trim_end_matches(':');
                let selected = format!(
                    "if: (github.event_name != 'pull_request' || github.head_ref == 'develop' || startsWith(github.head_ref, 'release/')) && (!inputs.jobs || contains(format(',{{0}},', inputs.jobs), ',{job},'))"
                );
                let same_repository = format!(
                    "if: (github.event_name != 'pull_request' || ((github.head_ref == 'develop' || startsWith(github.head_ref, 'release/')) && github.event.pull_request.head.repo.full_name == github.repository)) && (!inputs.jobs || contains(format(',{{0}},', inputs.jobs), ',{job},'))"
                );
                assert!(
                    [
                        Some(guard),
                        Some(selected.as_str()),
                        Some(same_repository.as_str())
                    ]
                    .contains(&next),
                    "{name}: job {line} must run only for develop or release branches into main"
                );
            }
        }
    }
    let fuzz = fs::read_to_string(root.join(".github/workflows/fuzz-scheduled.yml"))?;
    assert!(
        !fuzz.contains("schedule:"),
        "fuzzing runs on demand, not on a schedule"
    );
    Ok(())
}

#[test]
fn self_hosted_runners_serve_only_opted_in_branch_pushes_and_dispatches()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    let guard = "(github.event_name == 'push' || github.event_name == 'workflow_dispatch') && startsWith(github.ref, 'refs/heads/') && vars.TACHYON_SELF_HOSTED == 'true'";
    let hosted_fallbacks = [
        "|| 'ubuntu-24.04' }}",
        "|| 'macos-latest' }}",
        "|| 'windows-latest' }}",
        "|| matrix.os }}",
    ];

    let runner_lines = workflow
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("runs-on:"))
        .collect::<Vec<_>>();
    assert!(!runner_lines.is_empty());
    for line in runner_lines {
        assert!(
            line.contains(guard),
            "every Rust CI job chooses a self-hosted runner only behind `{guard}`: {line}"
        );
        assert!(
            hosted_fallbacks
                .iter()
                .any(|fallback| line.ends_with(fallback)),
            "every Rust CI job falls back to a GitHub-hosted runner: {line}"
        );
    }

    for line in workflow
        .lines()
        .map(str::trim_start)
        .filter(|line| line.contains("self-hosted"))
    {
        let guarded_choice = line.starts_with("runs-on:") && line.contains(guard);
        let matrix_labels = line.starts_with("self_hosted_labels: '[\"self-hosted\",\"tachyon\",");
        assert!(
            guarded_choice || matrix_labels,
            "self-hosted labels appear only in guarded runner choices and matrix label entries: {line}"
        );
    }
    assert!(workflow.contains("vars.TACHYON_SELF_HOSTED_ANDROID == 'true'"));
    assert!(!workflow.contains("pull_request_target"));

    for other in collect_files(&root.join(".github/workflows"), "yml")? {
        if other.ends_with("rust-ci.yml") {
            continue;
        }
        let contents = fs::read_to_string(&other)?;
        assert!(
            !contents.contains("self-hosted"),
            "{} must stay on GitHub-hosted runners",
            other.display()
        );
    }
    Ok(())
}

fn function_bodies(source: &str) -> Vec<(String, String)> {
    let mut bodies: Vec<(String, String)> = Vec::new();
    for line in source.lines() {
        let signature = line
            .strip_prefix("    ")
            .filter(|rest| !rest.starts_with(' '))
            .and_then(|rest| rest.split_once("fn "))
            .map(|(_, rest)| rest.split(['(', '<']).next().unwrap_or("").to_owned());
        match (signature, bodies.last_mut()) {
            (Some(name), _) => bodies.push((name, format!("{line}\n"))),
            (None, Some((_, body))) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    bodies
}

#[test]
fn every_dispatch_path_reaches_the_route_guard_or_is_declared_public()
-> Result<(), Box<dyn std::error::Error>> {
    let server = repository_root().join("crates/tachyon-core/src/server");
    let dispatch = fs::read_to_string(server.join("dispatch.rs"))?;
    let topics = fs::read_to_string(server.join("topic_stream.rs"))?;
    let route_guard = "RouteGuard::refusal(";
    let guarded = [
        (
            "dispatch.rs",
            &dispatch,
            route_guard,
            "SocketSession::open(",
        ),
        (
            "dispatch.rs",
            &dispatch,
            route_guard,
            "self.serve_allowed_route(",
        ),
        (
            "dispatch.rs",
            &dispatch,
            route_guard,
            "Self::options_or_refusal(",
        ),
        (
            "topic_stream.rs",
            &topics,
            "Self::refusal(",
            "Self::subscribe(",
        ),
    ];
    for (file, source, guard, sink) in guarded {
        let callers: Vec<(String, String)> = function_bodies(source)
            .into_iter()
            .filter(|(_, body)| body.contains(sink))
            .collect();
        assert!(!callers.is_empty(), "{file} no longer calls `{sink}`");
        for (name, body) in callers {
            let first_guard = body.find(guard);
            let call = body.find(sink);
            assert!(
                first_guard.zip(call).is_some_and(|(at, call)| at <= call),
                "{file}: `{name}` calls `{sink}` without `{guard}` first"
            );
        }
    }
    let only_from = [
        (&dispatch, "RouteSchema::for_path(", "options_for"),
        (&dispatch, "Self::options_for(", "options_or_refusal"),
        (
            &dispatch,
            "self.clone().serve_public_static(",
            "serve_found",
        ),
        (&dispatch, "TopicStream::open(", "serve_path"),
        (&topics, "Self::open_declared(", "open"),
        (&topics, "RouteGuard::refusal(state, access", "refusal"),
    ];
    for (source, call, owner) in only_from {
        let callers: Vec<String> = function_bodies(source)
            .into_iter()
            .filter(|(name, body)| name != owner && body.contains(call))
            .map(|(name, _)| name)
            .collect();
        assert!(
            callers.is_empty(),
            "`{call}` is reached from {callers:?}, not only from `{owner}`"
        );
    }
    let declared_public = [
        "HotUpdateHub::LIVE_ENDPOINT",
        "HotUpdateHub::HOT_ENDPOINT",
        "HotUpdateHub::DIAGNOSTICS_ENDPOINT",
        "HotUpdateHub::LIVE_SCRIPT_ENDPOINT",
    ];
    let guarded_prefixes = ["TopicHub::ENDPOINT"];
    for line in dispatch.lines() {
        let compared = line
            .split_once("path == ")
            .or_else(|| line.split_once(".strip_prefix("))
            .map(|(_, rest)| rest.split([' ', ')', '&']).next().unwrap_or(""));
        if let Some(endpoint) = compared {
            assert!(
                declared_public.contains(&endpoint) || guarded_prefixes.contains(&endpoint),
                "dispatch.rs serves `{endpoint}`, which is neither guarded nor declared public"
            );
        }
    }
    Ok(())
}

#[test]
fn the_published_api_reference_shows_only_what_the_route_guard_allows_anonymously()
-> Result<(), Box<dyn std::error::Error>> {
    let core = repository_root().join("crates/tachyon-core/src");
    let reference = fs::read_to_string(core.join("handler/api_reference.rs"))?;
    let site_build = fs::read_to_string(core.join("compiler/site_build.rs"))?;
    let published = function_bodies(&reference)
        .into_iter()
        .find(|(name, _)| name == "published_files")
        .map(|(_, body)| body)
        .ok_or("api_reference.rs no longer has `published_files`")?;
    assert!(
        published.contains("Self::controller_routes(graph, RouteGuard::allows_anonymous"),
        "`published_files` must list routes through `RouteGuard::allows_anonymous`"
    );
    let contract_callers: Vec<String> = function_bodies(&site_build)
        .into_iter()
        .filter(|(_, body)| body.contains("ApiReference::contract("))
        .map(|(name, body)| (name, body.contains("ApiBaselineCheck::verify(")))
        .filter(|(_, verifies)| !verifies)
        .map(|(name, _)| name)
        .collect();
    assert!(
        contract_callers.is_empty(),
        "site_build.rs uses the full API contract outside the baseline check in {contract_callers:?}"
    );
    Ok(())
}

#[test]
fn ci_runs_the_browser_gates_that_exist() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;

    for step in [
        "bun run test:authored-node-browser",
        "bun run test:svg-namespace-browser",
        "bun run test:component-scope-browser",
        "bun run test:logic-controls-browser",
        "bun run test:scaffold-browser",
        "node website/scripts/build-styles.mjs --check",
        "node website/scripts/language-coverage.mjs --strict",
        "node website/scripts/check-responsive.mjs",
        "node website/scripts/check-migration.mjs",
        "node scripts/ignored-browser-gates.mjs --groups=dotnet,kotlin,java",
        "TAC_KOTLIN_WEB_REQUIRE_TOOLCHAINS: \"1\"",
        "TAC_BROWSER_REQUIRE_TOOLCHAINS: \"1\"",
        "run: bun run test:${{ matrix.gate }}",
    ] {
        assert!(workflow.contains(step), "CI does not run `{step}`");
    }
    let package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("package.json"))?)?;
    let scripts = package["scripts"].as_object().cloned().unwrap_or_default();
    let unrun = scripts
        .keys()
        .filter(|name| name.ends_with("-browser"))
        .filter(|name| {
            let step = format!("bun run {name}");
            let gate = format!("          - {}\n", name.trim_start_matches("test:"));
            [workflow.contains(&step), workflow.contains(&gate)] == [false, false]
        })
        .collect::<Vec<_>>();
    assert!(
        unrun.is_empty(),
        "CI runs no step or browser-languages gate for {unrun:?}"
    );
    Ok(())
}

#[test]
fn completed_phase_acceptance_is_enforced_by_ci_and_documentation()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/rust-ci.yml"))?;
    let plan = fs::read_to_string(root.join("docs/PROJECT_PLAN.md"))?;
    let version = fs::read_to_string(root.join("VERSION"))?;

    assert!(workflow.contains("Run unit and Phase 4 real-process acceptance tests"));
    assert!(workflow.contains("actions/setup-node@820762786026740c76f36085b0efc47a31fe5020"));
    assert!(workflow.contains("node-version: 24.22.2"));
    assert!(workflow.contains("actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97"));
    assert!(workflow.contains("python-version: 3.14.6"));
    assert!(workflow.contains("ruby/setup-ruby@14594264cd68ce8a2345dd349bc3d138a4ef85c8"));
    assert!(workflow.contains("ruby-version: \"4.0.6\""));
    assert!(workflow.contains("cargo run --release --locked --bin ty -- --version"));
    assert!(workflow.contains("--fail-under-lines 80"));
    assert!(workflow.contains("--fail-under-functions 80"));
    assert!(workflow.contains("--fail-under-regions 80"));
    assert!(workflow.contains(
        "--ignore-filename-regex 'crates/tachyon-core/src/native/(compiler|macos)\\.rs'"
    ));
    assert!(workflow.contains("Phase 3 Chromium component and service behavior"));
    assert!(workflow.contains("bun run test:phase3-browser"));
    assert!(workflow.contains("Phase 4 macOS native and mobile-web parity"));
    assert!(workflow.contains("bun run test:phase4-macos"));
    assert!(plan.contains("## Phase 3: Tac View Semantics"));
    assert!(plan.contains("## Phase 4: Native Vertical Slice"));
    assert!(plan.matches("Status: complete on 2026-07-26").count() >= 4);
    assert_eq!(version.trim(), "26.45.01");
    assert_eq!(version.trim(), tachyon_contracts::PRODUCT_VERSION);
    Ok(())
}

#[test]
fn cohesion_violations_cannot_grow_above_the_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let baseline: usize = fs::read_to_string(root.join("tools/cohesion-scan/BASELINE"))?
        .trim()
        .parse()?;
    let report = cohesion_scan::Scan::repository(&root)?;
    assert!(
        report.total <= baseline,
        "cohesion violations grew from {baseline} to {}; run `cargo run -p cohesion-scan` to list them\n{}",
        report.total,
        report.table()
    );
    Ok(())
}

#[test]
fn duplicated_blocks_cannot_grow_above_the_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let baseline: usize =
        fs::read_to_string(root.join("tools/cohesion-scan/DUPLICATION_BASELINE"))?
            .trim()
            .parse()?;
    let report = cohesion_scan::Scan::repository(&root)?;
    assert!(
        report.duplication_total <= baseline,
        "duplicated blocks grew from {baseline} to {}; move the repeated code into one static method of a utility type (ADR 0064) and run `cargo run -p cohesion-scan` to list them\n{}",
        report.duplication_total,
        report.duplicates.join("\n")
    );
    Ok(())
}

#[test]
fn structure_violations_cannot_grow_above_the_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let baseline: usize = fs::read_to_string(root.join("tools/cohesion-scan/STRUCTURE_BASELINE"))?
        .trim()
        .parse()?;
    let report = cohesion_scan::Scan::repository(&root)?;
    assert!(
        report.structure_total <= baseline,
        "files with more than one type, free functions or file-level values grew from {baseline} to {}; run `cargo run -p cohesion-scan` to list them\n{}",
        report.structure_total,
        report.table()
    );
    Ok(())
}

#[test]
fn file_name_violations_cannot_grow_above_the_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let baseline: usize = fs::read_to_string(root.join("tools/cohesion-scan/FILE_NAME_BASELINE"))?
        .trim()
        .parse()?;
    let report = cohesion_scan::Scan::repository(&root)?;
    assert!(
        report.file_name_total <= baseline,
        "files not named after their one type grew from {baseline} to {}; name each Rust file after its type in snake_case (ADR 0067) and run `cargo run -p cohesion-scan` to list them\n{}",
        report.file_name_total,
        report.table()
    );
    Ok(())
}
