# fs-test

A simple test component for verifying filesystem access via `--allow-dir`.

Exports one function `increment-counter` that reads a counter from
`{dir}/counter.txt`, increments it by one, and writes it back. If the
file doesn't exist it starts at 0.

## Setup

```bash
asterai component build
asterai env init fs-test-env
asterai env add-component fs-test-env asterai:fs-test
```

## Usage

```bash
asterai env call fs-test-env --allow-dir ~/fs-test \
  asterai:fs-test fs-test/increment-counter ~/fs-test
```

Run it multiple times to see the counter increment.

## Note on tilde (~) expansion

The shell only expands `~` when it is **unquoted**. Inside double quotes
it is passed as a literal character, which WASI cannot resolve:

```bash
# Works — shell expands ~ before the CLI sees it:
asterai env call ... ~/fs-test

# Broken — component receives literal "~/fs-test":
asterai env call ... "~/fs-test"

# Also works — $HOME expands inside double quotes:
asterai env call ... "$HOME/fs-test"
```

This applies to both `--allow-dir` and function arguments.
