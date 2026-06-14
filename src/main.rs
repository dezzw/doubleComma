#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Backend {
    Uv,
    Npm,
}

impl Backend {
    fn name(self) -> &'static str {
        match self {
            Self::Uv => "uv",
            Self::Npm => "npm",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Project {
    root: PathBuf,
    marker: &'static str,
    backend: Backend,
}

#[derive(Debug, Eq, PartialEq)]
struct CommandPlan {
    argv: Vec<OsString>,
    needed: bool,
    skip_reason: Option<&'static str>,
}

impl CommandPlan {
    fn new(argv: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
            needed: true,
            skip_reason: None,
        }
    }

    fn conditional(
        argv: impl IntoIterator<Item = impl Into<OsString>>,
        needed: bool,
        skip_reason: &'static str,
    ) -> Self {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
            needed,
            skip_reason: (!needed).then_some(skip_reason),
        }
    }
}

#[derive(Debug)]
enum CliError {
    NoProjectRoot(PathBuf),
    UnsupportedCommand {
        command: String,
        backend: Backend,
    },
    MissingRunArgs,
    CurrentDir(io::Error),
    ChangeDir {
        path: PathBuf,
        source: io::Error,
    },
    Execute {
        command: OsString,
        source: io::Error,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProjectRoot(path) => {
                write!(
                    out,
                    "error: no supported project root found from {}",
                    path.display()
                )
            }
            Self::UnsupportedCommand { command, backend } => write!(
                out,
                "error: command '{}' is not supported for backend {}",
                command,
                backend.name()
            ),
            Self::MissingRunArgs => write!(out, "error: ,, run requires at least one argument"),
            Self::CurrentDir(source) => {
                write!(out, "error: failed to read current directory: {source}")
            }
            Self::ChangeDir { path, source } => write!(
                out,
                "error: failed to change directory to {}: {source}",
                path.display()
            ),
            Self::Execute { command, source } => write!(
                out,
                "error: failed to execute {}: {source}",
                command.to_string_lossy()
            ),
        }
    }
}

fn path_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().try_exists().unwrap_or(false)
}

fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn package_script_exists(project: &Project, target: &str) -> bool {
    let Some(contents) = read_to_string(project.root.join("package.json")) else {
        return false;
    };

    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return false;
    };

    manifest
        .get("scripts")
        .and_then(|scripts| scripts.as_object())
        .and_then(|scripts| scripts.get(target))
        .and_then(|script| script.as_str())
        .is_some()
}

fn pyproject_script_exists(project: &Project, target: &str) -> bool {
    let Some(contents) = read_to_string(project.root.join("pyproject.toml")) else {
        return false;
    };

    let Ok(manifest) = contents.parse::<toml::Value>() else {
        return false;
    };

    manifest
        .get("project")
        .and_then(|project| project.get("scripts"))
        .and_then(|scripts| scripts.as_table())
        .and_then(|scripts| scripts.get(target))
        .is_some()
}

fn has_project_marker(path: &Path) -> bool {
    path_exists(path.join("uv.toml"))
        || path_exists(path.join("pyproject.toml"))
        || path_exists(path.join("package.json"))
}

fn find_project_root(start: impl Into<PathBuf>) -> Option<PathBuf> {
    let mut current = start.into();

    loop {
        if has_project_marker(&current) {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

fn detect_project(start: impl Into<PathBuf>) -> Option<Project> {
    let root = find_project_root(start)?;

    if path_exists(root.join("uv.toml")) {
        return Some(Project {
            root,
            marker: "uv.toml",
            backend: Backend::Uv,
        });
    }

    if path_exists(root.join("pyproject.toml")) {
        return Some(Project {
            root,
            marker: "pyproject.toml",
            backend: Backend::Uv,
        });
    }

    if path_exists(root.join("package.json")) {
        return Some(Project {
            root,
            marker: "package.json",
            backend: Backend::Npm,
        });
    }

    None
}

fn build_prepare_plan(project: &Project) -> CommandPlan {
    match project.backend {
        Backend::Uv => {
            let needed = !path_exists(project.root.join(".venv"));
            CommandPlan::conditional(["uv", "sync"], needed, ".venv already exists")
        }
        Backend::Npm => {
            let needed = !path_exists(project.root.join("node_modules"));
            if path_exists(project.root.join("package-lock.json")) {
                CommandPlan::conditional(["npm", "ci"], needed, "node_modules already exists")
            } else {
                CommandPlan::conditional(["npm", "install"], needed, "node_modules already exists")
            }
        }
    }
}

fn with_extra_args(
    argv: impl IntoIterator<Item = impl Into<OsString>>,
    args: &[OsString],
) -> CommandPlan {
    let mut argv = argv.into_iter().map(Into::into).collect::<Vec<_>>();
    argv.extend(args.iter().cloned());
    CommandPlan::new(argv)
}

fn npm_run_script_plan(target: &str, args: &[OsString]) -> CommandPlan {
    let mut argv = vec![
        OsString::from("npm"),
        OsString::from("run"),
        OsString::from(target),
    ];
    if !args.is_empty() {
        argv.push(OsString::from("--"));
        argv.extend(args.iter().cloned());
    }
    CommandPlan::new(argv)
}

fn npm_exec_plan(target: &str, args: &[OsString]) -> CommandPlan {
    let mut argv = vec![
        OsString::from("npm"),
        OsString::from("exec"),
        OsString::from("--no"),
        OsString::from("--"),
        OsString::from(target),
    ];
    argv.extend(args.iter().cloned());
    CommandPlan::new(argv)
}

fn npm_exec_args_plan(args: &[OsString]) -> CommandPlan {
    let mut argv = vec![
        OsString::from("npm"),
        OsString::from("exec"),
        OsString::from("--no"),
        OsString::from("--"),
    ];
    argv.extend(args.iter().cloned());
    CommandPlan::new(argv)
}

fn uv_run_plan(target: &str, args: &[OsString]) -> CommandPlan {
    let mut argv = vec![
        OsString::from("uv"),
        OsString::from("run"),
        OsString::from(target),
    ];
    argv.extend(args.iter().cloned());
    CommandPlan::new(argv)
}

fn uv_run_args_plan(args: &[OsString]) -> CommandPlan {
    let mut argv = vec![OsString::from("uv"), OsString::from("run")];
    argv.extend(args.iter().cloned());
    CommandPlan::new(argv)
}

fn resolve_existing_target_path(current_dir: &Path, target: &str) -> Option<PathBuf> {
    let target_path = Path::new(target);
    let absolute = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        current_dir.join(target_path)
    };

    absolute.canonicalize().ok()
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_os_string()
}

fn has_local_node_bin(project: &Project, name: &str) -> bool {
    path_exists(project.root.join("node_modules").join(".bin").join(name))
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn build_node_file_plan(project: &Project, path: &Path, args: &[OsString]) -> Option<CommandPlan> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("js" | "mjs" | "cjs") => {
            let mut argv = vec![OsString::from("node"), path_arg(path)];
            argv.extend(args.iter().cloned());
            Some(CommandPlan::new(argv))
        }
        Some("ts" | "tsx") if has_local_node_bin(project, "tsx") => {
            let mut argv = vec![
                OsString::from("npm"),
                OsString::from("exec"),
                OsString::from("--no"),
                OsString::from("--"),
                OsString::from("tsx"),
                path_arg(path),
            ];
            argv.extend(args.iter().cloned());
            Some(CommandPlan::new(argv))
        }
        Some("ts" | "tsx") if has_local_node_bin(project, "ts-node") => {
            let mut argv = vec![
                OsString::from("npm"),
                OsString::from("exec"),
                OsString::from("--no"),
                OsString::from("--"),
                OsString::from("ts-node"),
                path_arg(path),
            ];
            argv.extend(args.iter().cloned());
            Some(CommandPlan::new(argv))
        }
        _ if is_executable_file(path) => {
            let mut argv = vec![path_arg(path)];
            argv.extend(args.iter().cloned());
            Some(CommandPlan::new(argv))
        }
        _ => None,
    }
}

fn build_run_plan(
    project: &Project,
    current_dir: &Path,
    intent: &str,
    args: &[OsString],
) -> Option<CommandPlan> {
    if intent == "prepare" {
        return Some(build_prepare_plan(project));
    }

    if intent == "run" {
        return Some(match project.backend {
            Backend::Uv => uv_run_args_plan(args),
            Backend::Npm => npm_exec_args_plan(args),
        });
    }

    match project.backend {
        Backend::Uv => {
            if pyproject_script_exists(project, intent) {
                return Some(uv_run_plan(intent, args));
            }

            match intent {
                "test" => return Some(CommandPlan::new(["uv", "run", "pytest"])),
                "fmt" => return Some(CommandPlan::new(["uv", "run", "ruff", "format", "."])),
                "lint" => return Some(CommandPlan::new(["uv", "run", "ruff", "check", "."])),
                _ => {}
            }

            if let Some(path) = resolve_existing_target_path(current_dir, intent) {
                let mut argv = vec![OsString::from("uv"), OsString::from("run"), path_arg(&path)];
                argv.extend(args.iter().cloned());
                return Some(CommandPlan::new(argv));
            }

            Some(uv_run_plan(intent, args))
        }
        Backend::Npm => {
            if package_script_exists(project, intent) {
                return Some(npm_run_script_plan(intent, args));
            }

            match intent {
                "test" => return Some(with_extra_args(["npm", "test"], args)),
                "dev" | "build" | "fmt" | "lint" => {
                    return Some(npm_run_script_plan(intent, args));
                }
                _ => {}
            }

            if let Some(path) = resolve_existing_target_path(current_dir, intent) {
                return build_node_file_plan(project, &path, args);
            }

            Some(npm_exec_plan(intent, args))
        }
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let safe = value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':' | b'=')
    });

    if safe {
        return value.to_string();
    }

    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

fn command_to_shell(argv: &[OsString]) -> String {
    argv.iter()
        .map(|arg| shell_quote(&arg.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn explain_plan(project: &Project, plan: &CommandPlan) -> String {
    let mut output = format!(
        "root: {}\nbackend: {}\ncommand: {}\n",
        project.root.display(),
        project.backend.name(),
        command_to_shell(&plan.argv)
    );

    if let Some(reason) = plan.skip_reason {
        output.push_str(&format!("needed: no ({reason})\n"));
    }

    output
}

fn execute_command(argv: &[OsString]) -> Result<i32, CliError> {
    let Some((program, args)) = argv.split_first() else {
        return Ok(0);
    };

    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|source| CliError::Execute {
            command: program.clone(),
            source,
        })?;

    Ok(status.code().unwrap_or(1))
}

fn print_usage(mut out: impl Write) -> io::Result<()> {
    writeln!(
        out,
        "usage: ,, <command> [args...]\n\
         \n\
         commands:\n\
           detect\n\
           root\n\
           explain [intent] [args...]\n\
           prepare\n\
           test\n\
           fmt\n\
           lint\n\
           dev\n\
           build\n\
           run <args...>"
    )
}

fn run_cli(args: Vec<OsString>) -> Result<i32, CliError> {
    if args.len() < 2 {
        let _ = print_usage(io::stderr());
        return Ok(1);
    }

    let command = args[1].to_string_lossy();
    if matches!(command.as_ref(), "-h" | "--help" | "help") {
        let _ = print_usage(io::stdout());
        return Ok(0);
    }

    let current_dir = env::current_dir().map_err(CliError::CurrentDir)?;
    let project = detect_project(current_dir.clone())
        .ok_or_else(|| CliError::NoProjectRoot(current_dir.clone()))?;

    match command.as_ref() {
        "root" => {
            println!("{}", project.root.display());
            Ok(0)
        }
        "detect" => {
            println!("root: {}", project.root.display());
            println!("marker: {}", project.marker);
            println!("backend: {}", project.backend.name());
            Ok(0)
        }
        "explain" => {
            let intent = args
                .get(2)
                .map(|arg| arg.to_string_lossy().into_owned())
                .unwrap_or_else(|| "prepare".to_string());
            let plan_args = args.get(3..).unwrap_or_default();
            let plan = build_run_plan(&project, &current_dir, &intent, plan_args).ok_or({
                CliError::UnsupportedCommand {
                    command: intent,
                    backend: project.backend,
                }
            })?;
            print!("{}", explain_plan(&project, &plan));
            Ok(0)
        }
        "run" if args.len() < 3 => Err(CliError::MissingRunArgs),
        _ => {
            let plan_args = args.get(2..).unwrap_or_default();
            let plan =
                build_run_plan(&project, &current_dir, &command, plan_args).ok_or_else(|| {
                    CliError::UnsupportedCommand {
                        command: command.into_owned(),
                        backend: project.backend,
                    }
                })?;

            print!("{}", explain_plan(&project, &plan));
            if !plan.needed {
                return Ok(0);
            }

            env::set_current_dir(&project.root).map_err(|source| CliError::ChangeDir {
                path: project.root.clone(),
                source,
            })?;

            execute_command(&plan.argv)
        }
    }
}

fn main() -> ExitCode {
    match run_cli(env::args_os().collect()) {
        Ok(code) => ExitCode::from(code.clamp(0, u8::MAX as i32) as u8),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn project_at(root: PathBuf, backend: Backend) -> Project {
        Project {
            root,
            marker: match backend {
                Backend::Uv => "pyproject.toml",
                Backend::Npm => "package.json",
            },
            backend,
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            env::temp_dir().join(format!("doublecomma-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn npm_project(name: &str, package_json: &str) -> Project {
        let root = temp_root(name);
        fs::write(root.join("package.json"), package_json).unwrap();
        project_at(root, Backend::Npm)
    }

    fn uv_project(name: &str, pyproject: &str) -> Project {
        let root = temp_root(name);
        fs::write(root.join("pyproject.toml"), pyproject).unwrap();
        project_at(root, Backend::Uv)
    }

    #[test]
    fn quotes_shell_display_for_explain_output() {
        assert_eq!(shell_quote("simple/path"), "simple/path");
        assert_eq!(shell_quote("two words"), "'two words'");
        assert_eq!(shell_quote("can't"), "'can'\\''t'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn builds_compat_uv_run_plan_with_forwarded_args() {
        let project = uv_project("uv-run", "");
        let args = os_args(&["python", "-m", "my_package"]);
        let plan = build_run_plan(&project, &project.root, "run", &args).unwrap();

        assert_eq!(
            plan.argv,
            os_args(&["uv", "run", "python", "-m", "my_package"])
        );
    }

    #[test]
    fn builds_compat_npm_run_plan_with_forwarded_args() {
        let project = npm_project("npm-run", "{}");
        let args = os_args(&["eslint", "."]);
        let plan = build_run_plan(&project, &project.root, "run", &args).unwrap();

        assert_eq!(
            plan.argv,
            os_args(&["npm", "exec", "--no", "--", "eslint", "."])
        );
    }

    #[test]
    fn node_package_script_wins_and_forwards_args() {
        let project = npm_project(
            "npm-script",
            r#"{"scripts":{"build":"vite build","lint":"eslint ."}}"#,
        );
        let args = os_args(&["--watch"]);
        let plan = build_run_plan(&project, &project.root, "build", &args).unwrap();

        assert_eq!(
            plan.argv,
            os_args(&["npm", "run", "build", "--", "--watch"])
        );
    }

    #[test]
    fn node_dependency_tool_uses_npm_exec_no_install() {
        let project = npm_project("npm-tool", "{}");
        let args = os_args(&["."]);
        let plan = build_run_plan(&project, &project.root, "eslint", &args).unwrap();

        assert_eq!(
            plan.argv,
            os_args(&["npm", "exec", "--no", "--", "eslint", "."])
        );
    }

    #[test]
    fn node_javascript_file_from_subdirectory_uses_absolute_path() {
        let project = npm_project("npm-js-file", "{}");
        let src_dir = project.root.join("src");
        let cwd = project.root.join("nested");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        let file = src_dir.join("index.js");
        fs::write(&file, "console.log('ok');").unwrap();

        let plan = build_run_plan(&project, &cwd, "../src/index.js", &[]).unwrap();

        assert_eq!(plan.argv, vec![OsString::from("node"), path_arg(&file)]);
    }

    #[test]
    fn node_typescript_file_uses_local_tsx_when_available() {
        let project = npm_project("npm-ts-file", "{}");
        let bin_dir = project.root.join("node_modules").join(".bin");
        let src_dir = project.root.join("src");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(bin_dir.join("tsx"), "").unwrap();
        let file = src_dir.join("main.ts");
        fs::write(&file, "console.log('ok');").unwrap();

        let plan = build_run_plan(&project, &project.root, "./src/main.ts", &[]).unwrap();

        assert_eq!(
            plan.argv,
            vec![
                OsString::from("npm"),
                OsString::from("exec"),
                OsString::from("--no"),
                OsString::from("--"),
                OsString::from("tsx"),
                path_arg(&file),
            ]
        );
    }

    #[test]
    fn uv_project_script_wins_and_forwards_args() {
        let project = uv_project(
            "uv-script",
            "[project]\nname = \"demo\"\n[project.scripts]\ncli = \"demo:main\"\n",
        );
        let args = os_args(&["--help"]);
        let plan = build_run_plan(&project, &project.root, "cli", &args).unwrap();

        assert_eq!(plan.argv, os_args(&["uv", "run", "cli", "--help"]));
    }

    #[test]
    fn uv_dependency_tool_uses_uv_run() {
        let project = uv_project("uv-tool", "[project]\nname = \"demo\"\n");
        let args = os_args(&["-q"]);
        let plan = build_run_plan(&project, &project.root, "pytest", &args).unwrap();

        assert_eq!(plan.argv, os_args(&["uv", "run", "pytest", "-q"]));
    }

    #[test]
    fn uv_file_from_subdirectory_uses_absolute_path() {
        let project = uv_project("uv-file", "[project]\nname = \"demo\"\n");
        let scripts_dir = project.root.join("scripts");
        let cwd = project.root.join("nested");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        let file = scripts_dir.join("foo.py");
        fs::write(&file, "print('ok')").unwrap();

        let plan = build_run_plan(&project, &cwd, "../scripts/foo.py", &[]).unwrap();

        assert_eq!(
            plan.argv,
            vec![OsString::from("uv"), OsString::from("run"), path_arg(&file)]
        );
    }

    #[test]
    fn uv_legacy_test_fallback_still_uses_pytest() {
        let project = uv_project("uv-test", "[project]\nname = \"demo\"\n");
        let plan = build_run_plan(&project, &project.root, "test", &[]).unwrap();

        assert_eq!(plan.argv, os_args(&["uv", "run", "pytest"]));
    }
}
