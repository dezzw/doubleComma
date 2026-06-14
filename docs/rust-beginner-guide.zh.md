# Rust 入门指南

这份文档面向 Rust 初学者。目标不是一次讲完 Rust 的所有细节，而是帮你建立一套能读懂、能修改、能测试本项目代码的基础知识。

本项目是一个命令行工具：用户运行 `,, <target> [args...]`，程序会识别当前目录所属项目类型，然后生成并执行对应命令。代码量不大，适合作为 Rust 入门练习。

## 1. Rust 是什么

Rust 是一门系统编程语言，重点是：

- 内存安全：不需要垃圾回收器，也能避免大量悬垂指针、重复释放、数据竞争问题。
- 性能：编译成本地机器码，运行时开销低。
- 类型系统强：很多错误在编译期就会被发现。
- 工具链统一：`cargo` 同时负责构建、测试、依赖管理、格式化、发布等工作。

Rust 适合写：

- CLI 工具
- 服务端程序
- 嵌入式和系统软件
- 高性能库
- 需要可靠错误处理的基础设施代码

## 2. 安装与常用命令

通常 Rust 通过 `rustup` 安装：

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

本仓库也提供了 Nix 开发环境：

```sh
nix develop
```

常用 Cargo 命令：

```sh
cargo build
cargo build --release
cargo test
cargo fmt
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo run -- help
```

在本项目里，Nix 安装后的最终命令名是 `,,`：

```sh
nix profile install path:.
,, help
```

直接用 Cargo 构建时，二进制文件名是 `double_comma`，因为 Cargo 的 binary target name 必须是合法 Rust crate 标识符：

```sh
cargo build --release
./target/release/double_comma help
```

## 3. Rust 项目结构

一个典型 Rust 项目有这些文件：

```text
Cargo.toml
Cargo.lock
src/
  lib.rs
  main.rs
tests/
```

本项目当前结构：

```text
Cargo.toml
Cargo.lock
flake.nix
src/
  lib.rs
  main.rs
docs/
  rust-beginner-guide.zh.md
```

### Cargo.toml

`Cargo.toml` 是项目配置文件，类似 Node 项目的 `package.json`。

本项目里有：

```toml
[package]
name = "doubleComma"
version = "0.1.0"
edition = "2021"

[lib]
name = "double_comma"

[[bin]]
name = "double_comma"
path = "src/main.rs"

[dependencies]
serde_json = "1"
toml = "0.8"
```

含义：

- `[package]`：包的元信息。
- `[lib]`：库 crate 的名字。代码中用 `double_comma::run_cli(...)` 引用。
- `[[bin]]`：二进制入口。这里入口文件是 `src/main.rs`。
- `[dependencies]`：依赖库。这里用 `serde_json` 解析 `package.json`，用 `toml` 解析 `pyproject.toml`。

### Cargo.lock

`Cargo.lock` 锁定依赖版本，保证不同机器构建出来用的是同一套依赖版本。应用程序通常应该提交 `Cargo.lock`。

### src/main.rs

`main.rs` 是程序入口。本项目的 `main.rs` 很薄：

```rust
#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match double_comma::run_cli(env::args_os().collect()) {
        Ok(code) => ExitCode::from(code.clamp(0, u8::MAX as i32) as u8),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
```

它只做三件事：

1. 读取命令行参数。
2. 调用库里的 `run_cli(...)`。
3. 把结果转成进程退出码。

### src/lib.rs

`lib.rs` 放主要逻辑。本项目把业务代码放在 `lib.rs`，是为了：

- 让 `main.rs` 保持简单。
- 让测试可以直接测内部函数。
- 未来如果要复用 CLI 逻辑，不必依赖二进制入口。

## 4. Rust 最小程序

最小 Rust 程序：

```rust
fn main() {
    println!("hello");
}
```

Rust 函数用 `fn` 定义。`println!` 后面的 `!` 表示这是宏，不是普通函数。

运行：

```sh
cargo run
```

## 5. 变量与可变性

Rust 默认变量不可变：

```rust
let name = "doubleComma";
// name = "other"; // 编译错误
```

要修改变量，需要 `mut`：

```rust
let mut count = 0;
count += 1;
```

这个默认不可变的设计可以减少意外修改，尤其适合写可靠工具。

## 6. 基本类型

常见类型：

```rust
let n: i32 = 42;
let ok: bool = true;
let ch: char = 'a';
let text: &str = "hello";
let owned: String = String::from("hello");
```

`&str` 和 `String` 的区别很重要：

- `&str`：字符串切片，通常是借用，不拥有数据。
- `String`：拥有字符串数据，可以增长、修改。

本项目中常见字符串相关类型：

- `&str`：静态或借用的 UTF-8 文本。
- `String`：拥有的 UTF-8 文本。
- `OsString`：操作系统命令行参数，可能不是合法 UTF-8。
- `Path` / `PathBuf`：文件路径。

CLI 工具经常使用 `OsString` 和 `PathBuf`，因为命令行参数和路径不一定总是普通 UTF-8 字符串。

## 7. 所有权：Rust 最重要的概念

Rust 的核心规则：

1. 每个值都有一个 owner。
2. 同一时间只能有一个 owner。
3. owner 离开作用域时，值会被释放。

例子：

```rust
let a = String::from("hello");
let b = a;
// println!("{a}"); // 编译错误：a 的所有权已经移动给 b
println!("{b}");
```

这叫 move。

如果不想转移所有权，可以借用：

```rust
fn print_name(name: &String) {
    println!("{name}");
}

let name = String::from("doubleComma");
print_name(&name);
println!("{name}");
```

`&String` 是不可变借用。

更常见的写法是借用 `&str`：

```rust
fn print_name(name: &str) {
    println!("{name}");
}
```

## 8. 借用与引用

引用有两种：

```rust
let value = String::from("hello");
let read_only = &value;
```

可变引用：

```rust
let mut value = String::from("hello");
let editable = &mut value;
editable.push_str(" world");
```

借用规则：

- 可以有多个不可变引用。
- 或者有一个可变引用。
- 不能同时有可变引用和不可变引用。

这些规则在编译期检查，用来防止数据竞争和悬垂引用。

## 9. Option：可能没有值

Rust 没有 `null`。可能不存在的值用 `Option<T>`：

```rust
let value: Option<i32> = Some(3);
let missing: Option<i32> = None;
```

处理 `Option` 常用 `match`：

```rust
match value {
    Some(n) => println!("{n}"),
    None => println!("missing"),
}
```

本项目里：

```rust
fn find_project_root(start: impl Into<PathBuf>) -> Option<PathBuf>
```

意思是：如果找到项目根目录，返回 `Some(path)`；找不到就返回 `None`。

也可以用 `?` 提前返回：

```rust
fn detect_project(start: impl Into<PathBuf>) -> Option<Project> {
    let root = find_project_root(start)?;
    // ...
}
```

如果 `find_project_root(start)` 是 `None`，整个函数直接返回 `None`。

## 10. Result：可能失败

可能失败的操作用 `Result<T, E>`：

```rust
let result: Result<i32, String> = Ok(1);
let error: Result<i32, String> = Err("failed".to_string());
```

本项目里：

```rust
pub fn run_cli(args: Vec<OsString>) -> Result<i32, CliError>
```

含义：

- 成功：返回进程退出码 `i32`。
- 失败：返回 `CliError`。

错误传播常用 `?`：

```rust
let current_dir = env::current_dir().map_err(CliError::CurrentDir)?;
```

如果 `env::current_dir()` 失败，函数直接返回错误；如果成功，就取出当前目录。

## 11. match：模式匹配

`match` 类似更强大的 `switch`：

```rust
match project.backend {
    Backend::Uv => {
        // uv 项目
    }
    Backend::Npm => {
        // npm 项目
    }
}
```

Rust 要求 `match` 覆盖所有可能情况。这能减少漏处理分支的 bug。

## 12. if let 和 let else

处理单个模式时，可以用 `if let`：

```rust
if let Some(path) = resolve_existing_target_path(current_dir, intent) {
    return Some(command_plan_with_path(&["uv", "run"], &path, args));
}
```

如果只关心 `Some`，这种写法比完整 `match` 更短。

`let else` 适合提前返回：

```rust
let Some(contents) = read_to_string(project.root.join("package.json")) else {
    return false;
};
```

意思是：如果读不到文件，就返回 `false`；否则把内容绑定到 `contents`。

## 13. struct：定义数据结构

本项目里：

```rust
struct Project {
    root: PathBuf,
    marker: &'static str,
    backend: Backend,
}
```

它表示一个被识别出的项目：

- `root`：项目根目录。
- `marker`：识别项目用到的文件，比如 `package.json`。
- `backend`：项目类型，比如 `npm` 或 `uv`。

另一个重要结构：

```rust
struct CommandPlan {
    argv: Vec<OsString>,
    needed: bool,
    skip_reason: Option<&'static str>,
}
```

它表示即将执行的命令计划：

- `argv`：命令和参数。
- `needed`：是否需要执行。
- `skip_reason`：如果跳过，原因是什么。

## 14. enum：定义有限状态

本项目里：

```rust
enum Backend {
    Uv,
    Npm,
}
```

`Backend` 只有两种可能。相比字符串 `"uv"` / `"npm"`，enum 更安全：

- 不会拼错。
- `match` 可以强制覆盖所有分支。
- IDE 和编译器能更好地帮助你重构。

错误也适合用 enum：

```rust
pub enum CliError {
    NoProjectRoot(PathBuf),
    UnsupportedCommand {
        command: String,
        backend: &'static str,
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
```

每种错误有自己的数据，显示错误时可以精确处理。

## 15. impl：给类型添加方法

```rust
impl Backend {
    fn name(self) -> &'static str {
        match self {
            Self::Uv => "uv",
            Self::Npm => "npm",
        }
    }
}
```

这给 `Backend` 添加了 `name()` 方法：

```rust
project.backend.name()
```

`Self::Uv` 里的 `Self` 指当前 impl 的类型，也就是 `Backend`。

## 16. trait：共享能力

Rust 的 trait 类似接口。

本项目实现了：

```rust
impl fmt::Display for CliError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ...
    }
}
```

实现 `Display` 后，就可以这样打印错误：

```rust
eprintln!("{error}");
```

`{error}` 会调用 `Display`。

## 17. Vec：动态数组

`Vec<T>` 是动态数组。

本项目里命令参数是：

```rust
Vec<OsString>
```

例如：

```rust
["npm", "run", "build", "--", "--watch"]
```

会表示成一组 `OsString`，最终交给 `std::process::Command` 执行。

常见操作：

```rust
let mut values = Vec::new();
values.push(1);
values.push(2);
```

本项目里：

```rust
let mut argv = argv(prefix);
argv.extend(args.iter().cloned());
```

含义：

- 先把固定命令前缀放进去。
- 再把用户传入的参数追加进去。

## 18. Iterator：迭代器

Rust 很常用迭代器：

```rust
parts.iter().map(OsString::from).collect()
```

这段代码含义：

1. 遍历 `parts`。
2. 每个 `&str` 转成 `OsString`。
3. 收集成 `Vec<OsString>`。

本项目 helper：

```rust
fn argv(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(OsString::from).collect()
}
```

`&[&str]` 表示字符串切片数组的借用。

## 19. 模块和可见性

Rust 默认所有东西都是私有的。

```rust
fn internal() {}
```

只有加 `pub` 才能被外部 crate 使用：

```rust
pub fn run_cli(args: Vec<OsString>) -> Result<i32, CliError> {
    // ...
}
```

本项目只暴露很小的 API：

- `run_cli`
- `CliError`

其他类型如 `Project`、`CommandPlan`、`Backend` 都是内部细节。

这样做的好处是：以后可以重构内部实现，不影响外部调用者。

## 20. 为什么测试在 lib.rs 里

Rust 有两类常见测试：

### 单元测试

写在源码文件内部：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

单元测试可以访问私有函数。

本项目当前测试大多是 planner 内部测试，例如：

- `build_run_plan`
- `resolve_existing_target_path`
- `shell_quote`

所以放在 `src/lib.rs` 里是合理的。

### 集成测试

放在 `tests/*.rs`：

```text
tests/
  cli.rs
```

集成测试像外部用户一样使用这个 crate，只能访问 public API。

如果未来要测试真实 CLI 行为，比如 stdout、stderr、exit code，可以新增：

```text
tests/cli.rs
```

当前不把所有测试移到 `tests/`，是为了避免把内部函数都改成 `pub`。

## 21. unsafe 与本项目的安全策略

本项目两个入口文件都有：

```rust
#![forbid(unsafe_code)]
```

意思是：禁止使用 `unsafe`。

`unsafe` 不是坏东西，但它要求程序员手动保证一些编译器不能验证的条件。初学阶段，尤其是 CLI 工具，最好完全不使用 `unsafe`。

CI 里也有检查，确保源码里只有这两处：

```text
src/lib.rs:1:#![forbid(unsafe_code)]
src/main.rs:1:#![forbid(unsafe_code)]
```

## 22. 本项目执行流程

用户运行：

```sh
,, build --watch
```

大致流程：

1. `src/main.rs` 读取命令行参数。
2. 调用 `double_comma::run_cli(...)`。
3. `run_cli` 找当前目录。
4. `detect_project` 向上查找项目根目录。
5. 根据 marker 判断项目类型：
   - `uv.toml`
   - `pyproject.toml`
   - `package.json`
6. `build_run_plan` 生成命令计划。
7. `explain_plan` 打印将要执行的命令。
8. 切换到项目根目录。
9. `execute_command` 执行真实命令。

## 23. 项目识别逻辑

核心函数：

```rust
fn has_project_marker(path: &Path) -> bool {
    path_exists(path.join("uv.toml"))
        || path_exists(path.join("pyproject.toml"))
        || path_exists(path.join("package.json"))
}
```

这表示只要目录里有这些文件之一，就认为它是项目根目录。

查找根目录：

```rust
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
```

这段逻辑：

1. 从当前目录开始。
2. 检查是否有 marker 文件。
3. 如果没有，就进入父目录。
4. 一直找到根目录为止。

`current.pop()` 会把路径退到父目录。如果已经没有父目录，就返回 `false`。

## 24. 命令规划逻辑

本项目不直接把用户输入拼成 shell 字符串，而是构造 `argv`：

```rust
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
```

这样比拼接 shell 字符串可靠：

- 不需要自己处理 shell 转义。
- 空格参数不会被拆错。
- 更不容易引入命令注入问题。

例如：

```rust
["npm", "run", "build", "--", "--watch"]
```

会作为明确的参数数组传给系统。

## 25. Node/npm 规则

当前规则：

1. 如果 `package.json` 里有同名 script，运行：

   ```sh
   npm run <script> -- <args...>
   ```

2. 兼容旧关键词：

   ```sh
   ,, test
   ,, dev
   ,, build
   ,, fmt
   ,, lint
   ```

3. 如果 target 是存在的文件：

   - `.js` / `.mjs` / `.cjs` 用 `node <file>`
   - `.ts` / `.tsx` 优先用本地 `tsx`，否则用本地 `ts-node`
   - 其他可执行文件直接运行

4. 否则当作 dependency tool：

   ```sh
   npm exec --no -- <tool> <args...>
   ```

`--no` 用来避免 npm 尝试远程安装缺失包。

## 26. uv/Python 规则

当前规则：

1. 如果 `[project.scripts]` 有同名 script：

   ```sh
   uv run <script> <args...>
   ```

2. 兼容旧关键词：

   ```sh
   ,, test  -> uv run pytest
   ,, fmt   -> uv run ruff format .
   ,, lint  -> uv run ruff check .
   ```

3. 如果 target 是存在的文件：

   ```sh
   uv run <absolute-file-path> <args...>
   ```

4. 否则当作工具：

   ```sh
   uv run <tool> <args...>
   ```

## 27. 为什么使用 OsString 和 PathBuf

CLI 工具不要假设所有参数都是 UTF-8。

Rust 的 `String` 必须是合法 UTF-8，但操作系统路径和参数不一定是。因此本项目使用：

```rust
Vec<OsString>
PathBuf
&Path
```

这是写 CLI 工具时更稳妥的选择。

## 28. Nix flake 在本项目里的作用

`flake.nix` 提供：

- 默认 package
- 默认 app
- dev shell

本项目因为最终命令名想叫 `,,`，而 Cargo binary target 不能直接叫 `,,`，所以 Nix 在安装阶段重命名：

```nix
postInstall = ''
  mv "$out/bin/double_comma" "$out/bin/,,"
'';
```

这意味着：

```sh
nix build
./result/bin/,, help
```

可以直接运行最终命令名。

## 29. CI 检查

GitHub Actions 里跑：

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
nix flake check
nix build
```

含义：

- `fmt`：代码格式必须统一。
- `clippy`：更严格的 Rust lint。
- `test`：运行测试。
- `build --release`：确保 release 构建可用。
- `nix flake check`：检查 flake 输出。
- `nix build`：确保 Nix package 能构建。

## 30. 阅读本项目源码的建议顺序

建议按这个顺序读：

1. `src/main.rs`
2. `run_cli`
3. `detect_project`
4. `find_project_root`
5. `build_run_plan`
6. `build_node_file_plan`
7. `execute_command`
8. `#[cfg(test)] mod tests`

不要一开始就从文件第一行读到最后一行。先抓主流程，再看细节。

## 31. 如何做第一个小改动

建议练习：给 uv 增加一个旧关键词：

```sh
,, typecheck
```

让它映射到：

```sh
uv run mypy .
```

你需要改：

```rust
match intent {
    "test" => return Some(CommandPlan::new(["uv", "run", "pytest"])),
    "fmt" => return Some(CommandPlan::new(["uv", "run", "ruff", "format", "."])),
    "lint" => return Some(CommandPlan::new(["uv", "run", "ruff", "check", "."])),
    _ => {}
}
```

可以加一行：

```rust
"typecheck" => return Some(command_plan(&["uv", "run", "mypy", "."], args)),
```

然后补测试：

```rust
#[test]
fn uv_legacy_typecheck_uses_mypy() {
    let project = uv_project("uv-typecheck", "[project]\nname = \"demo\"\n");
    let plan = build_run_plan(&project, &project.root, "typecheck", &[]).unwrap();

    assert_eq!(plan.argv, os_args(&["uv", "run", "mypy", "."]));
}
```

最后运行：

```sh
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```

## 32. 常见编译错误怎么读

Rust 编译错误通常很长，但结构清晰：

```text
error[E0382]: borrow of moved value
```

先看：

1. error 类型。
2. 文件和行号。
3. 编译器指出的变量。
4. help 建议。

不要急着改代码。先理解是所有权、借用、生命周期、类型不匹配，还是可见性问题。

## 33. 初学者最常见问题

### 为什么不能直接改变量

因为变量默认不可变。加 `mut`：

```rust
let mut value = 1;
value += 1;
```

### 为什么 String 传进函数后不能再用

因为所有权 move 了。改成借用：

```rust
fn use_name(name: &str) {}
```

### 为什么不能返回局部变量的引用

局部变量函数结束就释放了，引用会悬垂。返回拥有值：

```rust
fn make_name() -> String {
    String::from("doubleComma")
}
```

### 为什么要用 Result 而不是 panic

CLI 工具应该把可预期错误返回给用户，而不是崩溃。比如找不到项目根目录就是正常错误。

## 34. 继续学习路线

建议按顺序学习：

1. 变量、函数、基础类型。
2. 所有权、借用、引用。
3. `Option` / `Result`。
4. `struct` / `enum` / `match`。
5. `impl` / trait。
6. 模块和 crate。
7. 测试。
8. CLI 参数和文件路径。
9. 错误处理库，如 `anyhow`、`thiserror`。
10. 异步 Rust，如 `tokio`。

对本项目来说，前 8 项已经足够完成大多数改动。

## 35. 小结

你现在需要记住的核心点：

- `main.rs` 是入口，`lib.rs` 是主要逻辑。
- Rust 默认不可变。
- 所有权决定值能在哪里使用。
- `Option` 表示可能没有值。
- `Result` 表示可能失败。
- `match` 让分支处理更安全。
- `Vec<OsString>` 适合 CLI 参数。
- `PathBuf` / `Path` 适合文件路径。
- 单元测试放在 `lib.rs` 可以测试私有函数。
- CI 会帮你守住格式、lint、测试和构建。

最好的入门方式是：每次只做一个小改动，写一个测试，然后跑 `cargo test` 和 `cargo clippy`。
