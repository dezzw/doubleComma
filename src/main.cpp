#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <system_error>
#include <utility>
#include <vector>

namespace fs = std::filesystem;

namespace {

enum class Backend {
    Uv,
    Npm,
};

struct Project {
    fs::path root;
    std::string marker;
    Backend backend;
};

struct CommandPlan {
    std::vector<std::string> argv;
    bool needed = true;
    std::string skip_reason;
};

std::string backend_name(Backend backend) {
    switch (backend) {
    case Backend::Uv:
        return "uv";
    case Backend::Npm:
        return "npm";
    }
    return "unknown";
}

bool path_exists(const fs::path& path) {
    std::error_code ec;
    return fs::exists(path, ec);
}

CommandPlan command(std::vector<std::string> argv) {
    return CommandPlan{std::move(argv), true, ""};
}

std::optional<fs::path> find_project_root(fs::path start) {
    start = fs::absolute(std::move(start));

    while (true) {
        if (path_exists(start / "uv.toml") || path_exists(start / "pyproject.toml") ||
            path_exists(start / "package.json")) {
            return start;
        }

        const fs::path parent = start.parent_path();
        if (parent == start || parent.empty()) {
            return std::nullopt;
        }
        start = parent;
    }
}

std::optional<Project> detect_project(const fs::path& start) {
    const auto root = find_project_root(start);
    if (!root.has_value()) {
        return std::nullopt;
    }

    if (path_exists(*root / "uv.toml")) {
        return Project{*root, "uv.toml", Backend::Uv};
    }
    if (path_exists(*root / "pyproject.toml")) {
        return Project{*root, "pyproject.toml", Backend::Uv};
    }
    if (path_exists(*root / "package.json")) {
        return Project{*root, "package.json", Backend::Npm};
    }

    return std::nullopt;
}

CommandPlan build_prepare_plan(const Project& project) {
    if (project.backend == Backend::Uv) {
        const bool needed = !path_exists(project.root / ".venv");
        return CommandPlan{{"uv", "sync"}, needed, needed ? "" : ".venv already exists"};
    }

    const bool needed = !path_exists(project.root / "node_modules");
    const bool has_lockfile = path_exists(project.root / "package-lock.json");
    if (has_lockfile) {
        return CommandPlan{{"npm", "ci"}, needed, needed ? "" : "node_modules already exists"};
    }
    return CommandPlan{{"npm", "install"}, needed, needed ? "" : "node_modules already exists"};
}

std::optional<CommandPlan> build_run_plan(const Project& project,
                                          std::string_view intent,
                                          const std::vector<std::string>& args) {
    if (intent == "prepare") {
        return build_prepare_plan(project);
    }

    if (project.backend == Backend::Uv) {
        if (intent == "test") {
            return command({"uv", "run", "pytest"});
        }
        if (intent == "fmt") {
            return command({"uv", "run", "ruff", "format", "."});
        }
        if (intent == "lint") {
            return command({"uv", "run", "ruff", "check", "."});
        }
        if (intent == "run") {
            std::vector<std::string> argv{"uv", "run"};
            argv.insert(argv.end(), args.begin(), args.end());
            return command(std::move(argv));
        }
        return std::nullopt;
    }

    if (intent == "test") {
        return command({"npm", "test"});
    }
    if (intent == "dev") {
        return command({"npm", "run", "dev"});
    }
    if (intent == "build") {
        return command({"npm", "run", "build"});
    }
    if (intent == "fmt") {
        return command({"npm", "run", "fmt"});
    }
    if (intent == "lint") {
        return command({"npm", "run", "lint"});
    }
    if (intent == "run") {
        std::vector<std::string> argv{"npm", "exec", "--"};
        argv.insert(argv.end(), args.begin(), args.end());
        return command(std::move(argv));
    }

    return std::nullopt;
}

std::string shell_quote(const std::string& value) {
    if (value.empty()) {
        return "''";
    }

    bool safe = true;
    for (const char ch : value) {
        const bool is_alnum = (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9');
        const bool is_safe_symbol = ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == ':' || ch == '=';
        if (!is_alnum && !is_safe_symbol) {
            safe = false;
            break;
        }
    }

    if (safe) {
        return value;
    }

    std::string quoted = "'";
    for (const char ch : value) {
        if (ch == '\'') {
            quoted += "'\\''";
        } else {
            quoted += ch;
        }
    }
    quoted += "'";
    return quoted;
}

std::string command_to_shell(const std::vector<std::string>& argv) {
    std::ostringstream out;
    for (std::size_t i = 0; i < argv.size(); ++i) {
        if (i > 0) {
            out << ' ';
        }
        out << shell_quote(argv[i]);
    }
    return out.str();
}

std::string explain_plan(const Project& project, const CommandPlan& plan) {
    std::ostringstream out;
    out << "root: " << project.root.string() << '\n';
    out << "backend: " << backend_name(project.backend) << '\n';
    out << "command: " << command_to_shell(plan.argv) << '\n';
    if (!plan.needed) {
        out << "needed: no (" << plan.skip_reason << ")\n";
    }
    return out.str();
}

int execute_command(const std::vector<std::string>& argv) {
    return std::system(command_to_shell(argv).c_str());
}

void print_usage(std::ostream& out) {
    out << "usage: ,, <command> [args...]\n"
        << "\n"
        << "commands:\n"
        << "  detect\n"
        << "  root\n"
        << "  explain [intent] [args...]\n"
        << "  prepare\n"
        << "  test\n"
        << "  fmt\n"
        << "  lint\n"
        << "  dev\n"
        << "  build\n"
        << "  run <args...>\n";
}

std::vector<std::string> collect_args(int argc, char* argv[], int start) {
    std::vector<std::string> args;
    for (int i = start; i < argc; ++i) {
        args.emplace_back(argv[i]);
    }
    return args;
}

int run_cli(int argc, char* argv[]) {
    if (argc < 2) {
        print_usage(std::cerr);
        return 1;
    }

    const std::string command = argv[1];
    if (command == "-h" || command == "--help" || command == "help") {
        print_usage(std::cout);
        return 0;
    }

    const auto project = detect_project(fs::current_path());
    if (!project.has_value()) {
        std::cerr << "error: no supported project root found from " << fs::current_path() << '\n';
        return 1;
    }

    if (command == "root") {
        std::cout << project->root.string() << '\n';
        return 0;
    }

    if (command == "detect") {
        std::cout << "root: " << project->root.string() << '\n';
        std::cout << "marker: " << project->marker << '\n';
        std::cout << "backend: " << backend_name(project->backend) << '\n';
        return 0;
    }

    if (command == "explain") {
        const std::string intent = argc >= 3 ? argv[2] : "prepare";
        const std::vector<std::string> args = collect_args(argc, argv, 3);
        const auto plan = build_run_plan(*project, intent, args);
        if (!plan.has_value()) {
            std::cerr << "error: command '" << intent << "' is not supported for backend "
                      << backend_name(project->backend) << '\n';
            return 1;
        }
        std::cout << explain_plan(*project, *plan);
        return 0;
    }

    if (command == "run" && argc < 3) {
        std::cerr << "error: ,, run requires at least one argument\n";
        return 1;
    }

    const std::vector<std::string> args = collect_args(argc, argv, 2);
    const auto plan = build_run_plan(*project, command, args);
    if (!plan.has_value()) {
        std::cerr << "error: command '" << command << "' is not supported for backend "
                  << backend_name(project->backend) << '\n';
        return 1;
    }

    std::cout << explain_plan(*project, *plan);
    if (!plan->needed) {
        return 0;
    }

    std::error_code ec;
    fs::current_path(project->root, ec);
    if (ec) {
        std::cerr << "error: failed to change directory to " << project->root << ": " << ec.message() << '\n';
        return 1;
    }

    return execute_command(plan->argv);
}

} // namespace

int main(int argc, char* argv[]) {
    return run_cli(argc, argv);
}
