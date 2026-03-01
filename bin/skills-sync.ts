/**
 * CLI entry point for skills-sync.
 * Wires the full pipeline: parse manifest -> read state -> diff -> execute -> report.
 *
 * Commands/flags:
 *   --global     sync to global scope (~/.config/skills-sync/skills.toml)
 *   --local      sync to project scope (default, CWD/.skills.toml)
 *   --dry-run    show planned changes without executing
 *   --init       create sample .skills.toml and exit
 *   --config     path to manifest file (overrides discovery)
 *   -y/--yes     skip confirmation prompts; pass -y to npx skills subcommands
 *   --verbose    show the exact npx skills commands being executed
 *   --agent      target a specific agent for all installs and removals
 *   -V/--version version flag (handled by commander)
 *   -h/--help    help flag (handled by commander)
 */

import { Command } from 'commander'
import { createRequire } from 'module'
import { confirm } from '@inquirer/prompts'
import { readManifest } from '../src/manifest.js'
import { readGlobalState, readLocalState } from '../src/state.js'
import { computeDiff } from '../src/diff.js'
import { executeSync, removeSkills } from '../src/executor.js'
import { reporter } from '../src/reporter.js'
import { writeManifestTemplate } from '../src/init.js'
import { SkillsSyncError } from '../src/errors.js'
import { readSourcesFile, writeSourcesFile, detectInstalledSkills } from '../src/sources.js'
import type { SyncOptions } from '../src/types.js'

// Read version from package.json using createRequire for ESM compatibility
const require = createRequire(import.meta.url)
const pkg = require('../package.json') as { version: string }

const program = new Command()

program
  .name('skills-sync')
  .description('Declarative skill management: commit .skills.toml, run npx skills-sync')
  .version(pkg.version, '-V, --version')
  .option('--global', 'sync to global scope (~/.config/skills-sync/skills.toml)')
  .option('--local', 'sync to project scope (default: CWD/.skills.toml)')
  .option('--dry-run', 'show planned changes without executing')
  .option('--init', 'create a sample .skills.toml in current directory')
  .option('--config <path>', 'path to manifest file (overrides discovery)')
  .option('-y, --yes', 'skip confirmation prompts; pass -y to npx skills subcommands')
  .option('--verbose', 'show the exact npx skills commands being executed')
  .option('--agent <agent>', 'target a specific agent for all installs and removals')

program.action(async (options: {
  global?: boolean
  local?: boolean
  dryRun?: boolean
  init?: boolean
  config?: string
  yes?: boolean
  verbose?: boolean
  agent?: string
}) => {
  try {
    // FLAG-01: --global and --local are mutually exclusive
    if (options.global && options.local) {
      reporter.printError(
        '--global and --local are mutually exclusive',
        'Use --global for global scope or omit for local scope (default)',
      )
      process.exit(1)
    }

    // Determine scope: global if --global, otherwise local (default)
    const scope = options.global ? 'global' : 'local'

    // --init: scaffold sample manifest and exit
    if (options.init) {
      const outputPath = await writeManifestTemplate(scope)
      console.log(`Created ${outputPath}`)
      console.log("Edit it to add your skill sources, then run 'npx skills-sync' to sync.")
      process.exit(0)
    }

    // Build SyncOptions with all Phase 2 fields
    const syncOptions: SyncOptions = {
      scope,
      dryRun: options.dryRun ?? false,
      configPath: options.config,
      yes: options.yes ?? false,
      verbose: options.verbose ?? false,
      agent: options.agent,
    }

    // Sync pipeline
    const manifest = await readManifest(options.config, scope)
    const state = scope === 'global' ? await readGlobalState() : await readLocalState()
    const diff = computeDiff(manifest, state)

    // Nothing to do: everything already installed AND no orphans
    if (diff.toAdd.length === 0 && diff.toRemove.length === 0 && !options.dryRun) {
      for (const source of diff.unchanged) {
        reporter.printUnchanged(source)
      }
      reporter.printSummary(0, 0, diff.unchanged.length)
      process.exit(0)
    }

    // OUTP-02: Pre-sync diff preview
    if (diff.toAdd.length > 0 || diff.toRemove.length > 0) {
      reporter.printPreSyncPreview(diff)
    }

    // Orphan handling: confirm removal (unless --yes)
    let effectiveDiff = diff
    if (diff.toRemove.length > 0) {
      reporter.printOrphans(diff.toRemove)

      let shouldRemove = syncOptions.yes ?? false

      if (!shouldRemove) {
        if (!process.stdin.isTTY) {
          // Non-interactive: skip removal, warn user to re-run with --yes
          reporter.warnSkippingOrphans('stdin is not a TTY; use --yes to remove orphans in CI')
          effectiveDiff = { ...diff, toRemove: [] }
        } else {
          try {
            shouldRemove = await confirm({
              message: `Remove ${diff.toRemove.length} orphaned skill(s)?`,
              default: false,
            })
          } catch {
            // ExitPromptError (Ctrl+C) or other abort — treat as "no"
            shouldRemove = false
          }

          if (!shouldRemove) {
            effectiveDiff = { ...diff, toRemove: [] }
          }
        }
      }
    }

    // Execute installs
    const result = await executeSync(effectiveDiff, syncOptions)

    // Execute removals
    let removed = 0
    if (effectiveDiff.toRemove.length > 0) {
      removed = await removeSkills(effectiveDiff.toRemove, syncOptions)
    }

    // Source attribution write-back (local scope only, non-dry-run, only when something changed)
    if (scope === 'local' && !syncOptions.dryRun && (result.installed > 0 || removed > 0)) {
      // Read existing attribution
      const existingFile = await readSourcesFile()
      const bySource = new Map<string, string[]>()
      if (existingFile) {
        for (const [src, names] of Object.entries(existingFile.sources)) {
          bySource.set(src, names)
        }
      }

      if (result.installed > 0) {
        // Snapshot after all installs to find newly installed skills
        const currentSkills = await detectInstalledSkills()
        const previouslyAttributed = new Set<string>()
        for (const names of bySource.values()) {
          for (const n of names) previouslyAttributed.add(n)
        }
        const newSkills = currentSkills.filter(s => !previouslyAttributed.has(s))

        // Attribute new skills to added sources
        // For single source: precise. For multiple: shared attribution (safe for orphan detection)
        const addedSources = diff.toAdd
        for (const source of addedSources) {
          const existing = bySource.get(source) ?? []
          bySource.set(source, [...new Set([...existing, ...newSkills])])
        }
      }

      // Remove orphaned sources (skills that were removed this run)
      if (removed > 0) {
        const removedSet = new Set(effectiveDiff.toRemove)
        for (const [source, skills] of bySource) {
          const remaining = skills.filter(s => !removedSet.has(s))
          if (remaining.length === 0) {
            bySource.delete(source)
          } else {
            bySource.set(source, remaining)
          }
        }
      }

      await writeSourcesFile(bySource)
    }

    // Print summary
    if (options.dryRun) {
      reporter.printDryRunSummary(diff.toAdd.length, effectiveDiff.toRemove.length, diff.unchanged.length)
    } else {
      reporter.printSummary(result.installed, removed, result.unchanged)
    }

    // Exit with non-zero if some sources failed
    if (result.failed > 0) {
      process.exit(1)
    }
  } catch (err) {
    // Top-level error handler
    if (err instanceof SkillsSyncError) {
      reporter.printError(err.message, err.hint)
      process.exit(1)
    }

    // Unexpected error
    const message = err instanceof Error ? err.message : String(err)
    reporter.printError(message, 'This is an unexpected error. Please report it.')
    process.exit(1)
  }
})

program.parse()
