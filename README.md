# skills-sync

Declarative, reproducible skill management for Claude Code: commit one file, run one command, every teammate has the same skills.

## Install

```bash
npx skills-sync
```

No global installation needed. Run directly with `npx`.

## Quick Start

1. Generate a manifest:

   ```bash
   npx skills-sync --init
   ```

2. Edit `.skills.json` to add your skill sources:

   ```json
   {
     "skills": {
       "repos": [
         "anthropics/skills/computer-use",
         "anthropics/skills/web-search"
       ]
     }
   }
   ```

3. Sync skills:

   ```bash
   npx skills-sync
   ```

## Flags

| Flag | Description |
|------|-------------|
| `--init` | Create a sample `.skills.json` in current directory |
| `--check` | Exit 0 if in sync, exit 1 if out of sync (CI gate) |
| `--dry-run` | Show planned changes without executing |
| `--global` | Sync to global scope (`~/.config/skills-sync/skills.json`) |
| `--local` | Sync to project scope (default) |
| `--config <path>` | Path to manifest file |
| `-y, --yes` | Skip confirmation prompts |
| `--verbose` | Show `npx skills` commands being executed |
| `--agent <agent>` | Target a specific agent |
| `-V, --version` | Show version |
| `-h, --help` | Show help |

## CI Usage

Add to your CI pipeline to enforce skill consistency:

```yaml
- run: npx skills-sync --check
```

Exits with code 1 if skills are out of sync. Run `npx skills-sync --yes` to fix.

## License

MIT
