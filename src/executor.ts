/**
 * Subprocess invocation with dry-run boundary.
 *
 * This is the ONLY module that calls execa / spawns subprocesses.
 * The dry-run check is a single boundary here — no other module needs awareness of it.
 *
 * Locked decisions:
 *   - Always pass --yes to npx skills add (prevents TTY hang in CI)
 *   - Always pass --global when scope is global
 *   - Warn and continue on failure (do NOT abort remaining sources)
 *   - stderr: 'inherit' — NEVER suppress stderr from npx skills
 */

import { execa, ExecaError } from 'execa'
import type { Diff, SyncOptions, SyncResult } from './types.js'
import { reporter } from './reporter.js'

/**
 * Remove orphaned skills by calling npx skills remove.
 * Batches all skill names into a single subprocess call.
 *
 * @param skillNames - Skill directory names to remove (NOT source URLs)
 * @param options - Sync options (scope, dryRun, verbose, agent)
 * @returns Number of skills removed (0 on failure, skillNames.length on success)
 */
export async function removeSkills(
  skillNames: string[],
  options: SyncOptions
): Promise<number> {
  if (skillNames.length === 0) return 0

  const args = ['skills', 'remove', ...skillNames, '--yes']
  if (options.scope === 'global') args.push('--global')
  if (options.agent) args.push('--agent', options.agent)

  const cmd = `npx ${args.join(' ')}`
  if (options.verbose) reporter.printCommand(cmd)

  if (options.dryRun) {
    reporter.printDryRunCommand(cmd)
    return 0
  }

  for (const name of skillNames) {
    reporter.printRemoving(name)
  }

  try {
    await execa('npx', args, { stdout: 'inherit', stderr: 'inherit' })
    for (const name of skillNames) {
      reporter.printRemoved(name)
    }
    return skillNames.length
  } catch (err) {
    const reason = err instanceof ExecaError ? err.message : String(err)
    for (const name of skillNames) {
      reporter.warnRemoveFailed(name, reason)
    }
    return 0
  }
}

/**
 * Execute the sync: install all sources in diff.toAdd, print unchanged.
 *
 * @param diff    - Computed diff (toAdd, unchanged, toRemove)
 * @param options - Sync options (scope, dryRun)
 * @returns SyncResult with installed/failed/unchanged counts
 */
export async function executeSync(diff: Diff, options: SyncOptions): Promise<SyncResult> {
  let installed = 0
  let failed = 0

  for (const source of diff.toAdd) {
    reporter.printAction('installing', source)

    const args = ['skills', 'add', source, '--yes']
    if (options.scope === 'global') {
      args.push('--global')
    }

    if (options.agent) {
      args.push('--agent', options.agent)
    }

    const cmd = `npx ${args.join(' ')}`

    if (options.dryRun) {
      // Dry-run boundary: print command but do NOT invoke subprocess
      reporter.printDryRunCommand(cmd)
      continue
    }

    if (options.verbose) reporter.printCommand(cmd)

    try {
      await execa('npx', args, {
        stdout: 'inherit', // show npx skills output to user
        stderr: 'inherit', // CRITICAL: never suppress stderr
      })
      reporter.printSuccess(source)
      installed++
    } catch (err) {
      // Per locked decision: warn and continue with remaining sources
      const reason = err instanceof ExecaError ? err.message : String(err)
      reporter.warnSourceFailed(source, reason)
      failed++
    }
  }

  // Print all unchanged sources
  for (const source of diff.unchanged) {
    reporter.printUnchanged(source)
  }

  return {
    installed,
    failed,
    unchanged: diff.unchanged.length,
    removed: 0,
  }
}
