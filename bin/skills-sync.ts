/**
 * CLI entry point for skills-sync.
 * Wires the full pipeline: parse manifest -> read state -> diff -> execute -> report.
 *
 * Commands/flags (locked decisions from CONTEXT.md):
 *   --global     sync to global scope (~/.config/skills-sync/skills.toml)
 *   --local      sync to project scope (default, CWD/.skills.toml)
 *   --dry-run    show planned changes without executing
 *   --init       create sample .skills.toml and exit
 *   --config     path to manifest file (overrides discovery)
 *   -V/--version version flag (handled by commander)
 *   -h/--help    help flag (handled by commander)
 */

import { Command } from 'commander'
import { createRequire } from 'module'
import { readManifest } from '../src/manifest.js'
import { readGlobalState, readLocalState } from '../src/state.js'
import { computeDiff } from '../src/diff.js'
import { executeSync } from '../src/executor.js'
import { reporter } from '../src/reporter.js'
import { writeManifestTemplate } from '../src/init.js'
import { SkillsSyncError } from '../src/errors.js'

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

program.action(async (options: {
  global?: boolean
  local?: boolean
  dryRun?: boolean
  init?: boolean
  config?: string
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

    // Sync pipeline
    const manifest = await readManifest(options.config, scope)
    const state = scope === 'global' ? await readGlobalState() : await readLocalState()
    const diff = computeDiff(manifest, state)

    // Nothing to do: everything already installed
    if (diff.toAdd.length === 0 && !options.dryRun) {
      // Print unchanged sources
      for (const source of diff.unchanged) {
        reporter.printUnchanged(source)
      }
      reporter.printSummary(0, diff.unchanged.length)
      process.exit(0)
    }

    // Execute or preview
    const result = await executeSync(diff, {
      scope,
      dryRun: options.dryRun ?? false,
      configPath: options.config,
    })

    // Print summary
    if (options.dryRun) {
      reporter.printDryRunSummary(diff.toAdd.length, diff.unchanged.length)
    } else {
      reporter.printSummary(result.installed, result.unchanged)
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
