# rootcall

`rootcall` is a small project-aware command dispatcher. Run `,,` from any
subdirectory and it walks upward to find the nearest supported project root,
detects the project type, and dispatches common commands through the right tool.

This MVP supports:

- Python projects using `uv`
- Node projects using `npm`

## Build

```sh
mkdir -p build
cmake -S . -B build
cmake --build build
```

## Install

```sh
mkdir -p ~/.local/bin
cp ./build/,, ~/.local/bin/,,
```

Make sure `~/.local/bin` is on your `PATH`.

## Python with uv

Python projects are detected by `uv.toml` or `pyproject.toml`.

```sh
,, detect
,, root
,, explain test
,, prepare
,, test
,, fmt
,, lint
,, run python -m my_package
```

Command mapping:

- `,, prepare` -> `uv sync`
- `,, test` -> `uv run pytest`
- `,, fmt` -> `uv run ruff format .`
- `,, lint` -> `uv run ruff check .`
- `,, run <args...>` -> `uv run <args...>`

`,, prepare` runs only when `.venv` is missing.

## Node with npm

Node projects are detected by `package.json`.

```sh
,, detect
,, root
,, explain test
,, prepare
,, test
,, dev
,, build
,, fmt
,, lint
,, run eslint .
```

Command mapping:

- `,, prepare` -> `npm ci` when `package-lock.json` exists, otherwise `npm install`
- `,, test` -> `npm test`
- `,, dev` -> `npm run dev`
- `,, build` -> `npm run build`
- `,, fmt` -> `npm run fmt`
- `,, lint` -> `npm run lint`
- `,, run <args...>` -> `npm exec -- <args...>`

`,, prepare` runs only when `node_modules` is missing.

## Current limitations

- No config files.
- No plugin system.
- No trust system.
- No JSON or TOML parsing.
- No multi-backend project support.
- Uses `std::system` for execution.
- Shell quoting is basic POSIX-style quoting.
