# doubleComma

`doubleComma` is a small project-aware command dispatcher. Run `,,` from any
subdirectory and it walks upward to find the nearest supported project root,
detects the project type, and dispatches common commands through the right tool.

This MVP supports:

- Python projects using `uv`
- Node projects using `npm`

## Build

```sh
cargo build --release
```

## Install

```sh
mkdir -p ~/.local/bin
cp ./target/release/doubleComma ~/.local/bin/,,
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
,, cli --help
,, pytest -q
,, ./scripts/task.py
```

Command mapping:

- `,, prepare` -> `uv sync`
- `,, <script>` -> `uv run <script>` when `<script>` is defined in `[project.scripts]`
- `,, test` -> `uv run pytest`
- `,, fmt` -> `uv run ruff format .`
- `,, lint` -> `uv run ruff check .`
- `,, <file>` -> `uv run <absolute-file-path>`
- `,, <tool> [args...]` -> `uv run <tool> [args...]`
- `,, run <args...>` -> `uv run <args...>` for compatibility

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
,, eslint .
,, ./src/index.js
,, ./src/main.ts
```

Command mapping:

- `,, prepare` -> `npm ci` when `package-lock.json` exists, otherwise `npm install`
- `,, <script>` -> `npm run <script> -- [args...]` when `<script>` is defined in `package.json`
- `,, test` -> `npm test`
- `,, dev` -> `npm run dev`
- `,, build` -> `npm run build`
- `,, fmt` -> `npm run fmt`
- `,, lint` -> `npm run lint`
- `,, <file.js>` -> `node <absolute-file-path>`
- `,, <file.ts>` -> `npm exec --no -- tsx <absolute-file-path>` when local `tsx` exists, otherwise local `ts-node`
- `,, <tool> [args...]` -> `npm exec --no -- <tool> [args...]`
- `,, run <args...>` -> `npm exec --no -- <args...>` for compatibility

`,, prepare` runs only when `node_modules` is missing.

## Current limitations

- No config files.
- No plugin system.
- No trust system.
- No multi-backend project support.
- `explain` command rendering uses basic POSIX-style quoting.
