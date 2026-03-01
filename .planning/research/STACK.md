# Stack Research

**Domain:** TypeScript CLI tool — declarative skill manifest orchestration
**Researched:** 2026-03-01
**Confidence:** HIGH (core libraries verified via official GitHub + npm; version numbers confirmed)

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| TypeScript | 5.x (latest stable) | Language | Type-safety catches flag/option mismatches at compile time; the project's thin orchestration layer benefits more from types than raw JS; aligns with the `npx skills` ecosystem (TypeScript-first) |
| Node.js | 20+ (LTS) | Runtime | Commander v14 requires Node 20+; LTS guarantees long-term support; v20 ships with native `fetch`, reducing dependency count |
| commander | 14.0.3 | Argument/option parsing | 500M weekly downloads; zero dependencies; 180 KB install size; full TypeScript types; supports option groups (used for `--global`/`--local` mutual exclusivity help text); requires Node 20+ which matches our constraint |
| smol-toml | 1.6.0 | TOML manifest parsing | Most downloaded TOML parser on npm (~7M weekly); fully TOML 1.1.0 compliant; 2x faster than `@iarna/toml`; last release Dec 2025; zero dependencies; both ESM and CJS; correct error messages on malformed input |
| execa | 9.6.1 | Shell-out to `npx skills` | 114M weekly downloads; no shell injection risk (template literal API, no shell interpolation); promise-based; native TypeScript types; cross-platform; v9 is pure ESM but we use tsup to compile anyway |
| picocolors | 1.1.1 | Terminal color output | Both ESM and CJS support (unlike chalk v5 which is ESM-only); zero dependencies; 14x smaller than chalk; used by PostCSS, Vite, Stylelint — battle-tested in tooling contexts; sufficient for our needs (info/warn/error/success coloring) |
| tsup | 8.5.1 | Build/bundle for npm distribution | Zero-config TypeScript bundler; outputs both CJS and ESM from one command; handles `bin` entrypoints; generates `.d.ts` declarations; powered by esbuild (fast); the standard for TypeScript npm CLI packages in 2025 |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| ora | 9.3.0 | Spinner during `npx skills` execution | Use during any shell-out that takes >1s; shows the user something is happening while `npx skills add/remove` runs; ESM-native, TypeScript types included |
| vitest | 2.x (latest) | Unit and integration tests | Use for all tests; Jest-compatible API; fast; first-class TypeScript support without extra config; replaces Jest for TypeScript CLIs |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| tsup | Builds `dist/` for npm publication | Config: `entry: ['src/cli.ts']`, `format: ['esm', 'cjs']`, `dts: true`, `clean: true` |
| TypeScript | Compile-time checks | `tsconfig.json` with `"moduleResolution": "bundler"`, `"target": "ES2022"`, `"strict": true` |
| vitest | Test runner | No Babel, no ts-jest; works directly with TypeScript source |
| @types/node | Node.js type definitions | Required for `fs`, `process`, `path` usage in TypeScript |

## Installation

```bash
# Core runtime dependencies
npm install commander smol-toml execa picocolors ora

# Dev dependencies
npm install -D typescript tsup vitest @types/node
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| commander 14 | Optique (`@optique/core`) | If you need compile-time enforcement of mutually exclusive flags via discriminated unions; Optique is emerging (2025 article recommendation) but ecosystem is immature — commander's stability and 500M weekly downloads justify it for a 500 LOC tool |
| commander 14 | oclif | If building a plugin-based CLI platform (e.g., Heroku CLI scale); 12 MB install with 30+ deps is overkill for a thin orchestration wrapper |
| commander 14 | yargs | If you need declarative schema-based option parsing with `.argv` object; yargs is 850 KB vs commander's 180 KB; commander is simpler for a fixed flag surface |
| smol-toml | @iarna/toml | If you need streaming/async TOML parsing of very large files; @iarna/toml is unmaintained (last release 2021) — avoid unless forced |
| smol-toml | toml (npm package) | Never; `toml` package is unmaintained and only targets TOML 0.4 |
| picocolors | chalk v5 | If you need truecolor (16M colors) or complex multi-line ANSI handling; chalk v5 is ESM-only which forces your CLI to be pure ESM — manageable but adds friction with tsup dual-output |
| picocolors | chalk v4 | Chalk v4 is CJS-compatible but effectively abandoned in favor of v5; don't start new projects on it |
| execa | child_process (native) | Never for shell-out in a CLI; native child_process requires manual promise wrapping, error handling, and stdout capture; execa handles all of this safely |
| execa | zx | If writing shell scripts, not programmatic CLI logic; zx is shell-oriented (bash-like); execa is programmatic-first with better error types |
| tsup | tsc (raw compiler) | If publishing a library (not CLI) that needs no bundling; tsup is strictly better for CLIs with `bin` entrypoints |
| tsup | Rollup | More control over output but far more configuration; unnecessary for a ~500 LOC CLI |
| ora | listr2 | If running multiple parallel tasks with individual spinners; listr2 shines for workflow UIs; ora is simpler for single sequential operations |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| @iarna/toml | Last published 2021 (unmaintained); only TOML 0.5 compliant | smol-toml 1.6.0 |
| chalk v5 alone | ESM-only; breaks CJS consumers of your CLI; requires `"type": "module"` in package.json which complicates dual CJS/ESM output | picocolors (both CJS and ESM) |
| yargs | 850 KB with 7 dependencies for functionality commander provides at 180 KB with 0 dependencies | commander |
| oclif | 12 MB, 30+ deps; designed for multi-plugin platforms not thin wrappers | commander |
| shelljs | Slow (slowest in benchmarks); not actively maintained; spawns shell by default (injection risk) | execa |
| ts-node (in tests) | Slow startup; vitest handles TypeScript natively | vitest |
| jest | Requires babel or ts-jest for TypeScript; significantly slower than vitest for this project size | vitest |
| vorpal | Archived/unmaintained; interactive CLI framework for a different use case | commander |

## Stack Patterns by Variant

**If project grows beyond ~500 LOC with complex subcommands:**
- Consider adding `@commander-js/extra-typings` for stronger TypeScript inference on parsed options
- This gives compile-time guarantee that `opts.yes` is `boolean`, not `boolean | undefined`

**If CI environment doesn't support color:**
- picocolors respects `NO_COLOR` env var and `process.stdout.isTTY` automatically
- No extra configuration needed

**If user passes `--dry-run`:**
- Skip `execa` calls entirely; log the `npx skills add/remove` commands that would have run
- Keep dry-run logic in the orchestration layer, not inside execa wrappers

**If distributing as `npx skills-sync` (recommended):**
- `package.json` must have `"bin": { "skills-sync": "./dist/cli.cjs" }` pointing to CJS output
- Add `#!/usr/bin/env node` as first line of `src/cli.ts` (tsup preserves shebangs)
- Set `"files": ["dist"]` to avoid shipping source TypeScript

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| commander@14 | Node.js >= 20 | Hard requirement; v15 pre-release requires Node 22+ — stay on 14 |
| execa@9 | Node.js >= 18 | Pure ESM; tsup handles bundling so this is not a consumer problem |
| smol-toml@1.6 | Node.js >= 16 | No Node version constraint; works with both ESM and CJS |
| picocolors@1.1 | Node.js >= 6 | No constraints; CJS and ESM both exported |
| ora@9.3 | Node.js >= 18 | ESM-only; same bundling note as execa |
| tsup@8 | Node.js >= 18 | Build-time only; not a runtime dependency |
| vitest@2 | Node.js >= 18 | Dev-time only |

**Summary:** Target Node.js 20+ (LTS) as minimum. All packages in the stack are compatible with Node 20.

## Sources

- [commander GitHub releases](https://github.com/tj/commander.js/releases) — v14.0.3 confirmed, Node 20+ requirement verified (HIGH confidence)
- [smol-toml GitHub](https://github.com/squirrelchat/smol-toml) — v1.6.0 confirmed December 2025 (HIGH confidence)
- [smol-toml npm downloads via WebSearch](https://www.npmjs.com/package/smol-toml) — ~7M weekly downloads confirmed (MEDIUM confidence)
- [execa jsDocs.io](https://www.jsdocs.io/package/execa) — v9.6.1 confirmed (HIGH confidence)
- [picocolors GitHub](https://github.com/alexeyraspopov/picocolors) — v1.1.1, CJS+ESM support confirmed (HIGH confidence)
- [ora npm](https://www.npmjs.com/package/ora) — v9.3.0 confirmed, ESM-only, Node 18+ (HIGH confidence)
- [tsup npm](https://www.npmjs.com/package/tsup) — v8.5.1 confirmed (HIGH confidence)
- [Building CLI apps with TypeScript in 2026 - DEV Community](https://dev.to/hongminhee/building-cli-apps-with-typescript-in-2026-5c9d) — Optique as emerging alternative noted (MEDIUM confidence, single source)
- [Comparison of Node.js terminal color libraries - DEV Community](https://dev.to/webdiscus/comparison-of-nodejs-libraries-to-colorize-text-in-terminal-4j3a) — picocolors vs chalk benchmark data (MEDIUM confidence)
- [TypeScript ESM/CJS in 2025 - Liran Tal](https://lirantal.com/blog/typescript-in-2025-with-esm-and-cjs-npm-publishing) — ESM/CJS dual-output complexity confirmed (HIGH confidence, current source)

---
*Stack research for: TypeScript CLI skill manifest orchestration (skills-sync v2)*
*Researched: 2026-03-01*
