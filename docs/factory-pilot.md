# Local smoke test for skill-sync

This procedure builds skill-sync from a checkout and runs a single local dry-run.
It uses only a temporary directory and a local example skill. It does not install
anything, does not change your personal agent directories or home configuration,
and does not need a GitHub token. The dry-run itself makes no network requests.
The build may fetch Rust dependencies on a fresh checkout.

Run the commands from the repository root. The examples below assume a Unix-like
shell. The test was checked with `cargo build --locked`,
`target/debug/skill-sync --help`, and the dry-run commands shown here.

## 1. Build and check the CLI

```sh
cargo build --locked
target/debug/skill-sync --help
```

The build exits with code `0`. The help output exits with code `0` and starts
with:

```text
Sync Agent Skills from a manifest

Usage: skill-sync [OPTIONS] [COMMAND]

Commands:
  status  Inspect local installations without network access or filesystem changes
  ...
```

## 2. Prepare a temporary project and run a dry-run

The following block creates an isolated home directory and a temporary project
containing a local skill and its manifest. `HOME` and `XDG_CONFIG_HOME` point
inside the temporary directory, so no personal configuration is read or
written. The manifest selects the default Codex project destination
(`.agents/skills`), but the dry-run does not create it. Run the block in a
subshell so its shell options and cleanup trap do not affect your interactive
shell.

```sh
(
  set -eu
  repo_root="$PWD"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  printf 'temporary directory: %s\n' "$tmp"

  mkdir -p "$tmp/home" "$tmp/project/example-skill"

  cat > "$tmp/project/example-skill/SKILL.md" <<'EOF'
---
name: hello-smoke
description: Minimal local fixture for the smoke test.
---
# Hello smoke

This file is only used by the local dry-run.
EOF

  cat > "$tmp/project/.skills.toml" <<'EOF'
[sync]
scope = "project"
agents = ["codex"]
mode = "copy"

[skills]
repos = ["local:./example-skill"]
EOF

  set +e
  output="$(
    cd "$tmp/project"
    env HOME="$tmp/home" XDG_CONFIG_HOME="$tmp/home/.config" \
      "$repo_root/target/debug/skill-sync" --local --dry-run --cache-limit-mib 0
  )"
  status=$?
  set -e

  printf '%s\n' "$output"
  printf 'exit code: %s\n' "$status"
  test "$status" -eq 2
)
```

Expected output (the exact path varies because it is a temporary directory):

```text
temporary directory: /tmp/tmp.XXXXXXXX
Install: local:./example-skill -> /tmp/tmp.XXXXXXXX/project/.agents/skills
exit code: 2
```

Exit code `2` is the documented successful preview result: the dry-run found a
change pending. It is not an error, and no files were installed. Exit code `0`
would mean the target was already up to date; a fresh temporary project should
not reach that state, so the check requires `2`. Any other exit code, or a
missing `Install:` line, means the smoke test failed.

## 3. Clean up

The `trap` in step 2 removes the temporary directory when the subshell exits,
including when the final exit-code check fails. If the block is interrupted
before that cleanup runs, remove the directory printed as
`temporary directory:` at the start:

```sh
rm -rf /tmp/tmp.XXXXXXXX   # use the printed path, not this placeholder
```

The temporary directory contains the project, the local example skill, and the
isolated home directory. Removing it leaves no installed skill or configuration
behind.
