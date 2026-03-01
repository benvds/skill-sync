/**
 * Terminal output with status symbols and colors.
 * Uses picocolors for zero-dep color output (respects NO_COLOR).
 *
 * Status symbols (locked decision from CONTEXT.md):
 *   ✓  present/done  — green
 *   ✗  error/failed  — red
 *   ↓  installing    — cyan
 *   ○  pending/skip  — dim
 */

import pc from 'picocolors'

const SYMBOLS = {
  present: '✓',
  error: '✗',
  installing: '↓',
  pending: '○',
} as const

export const reporter = {
  /** Print a per-source action line (installing or skipped). */
  printAction(type: 'installing' | 'skipped', source: string): void {
    if (type === 'installing') {
      console.log(`  ${pc.cyan(SYMBOLS.installing)} ${source}`)
    } else {
      console.log(`  ${pc.dim(SYMBOLS.pending)} ${source}`)
    }
  },

  /** Print a green checkmark after a successful install. */
  printSuccess(source: string): void {
    console.log(`  ${pc.green(SYMBOLS.present)} ${source}`)
  },

  /** Print a dim checkmark for a source that is already installed. */
  printUnchanged(source: string): void {
    console.log(`  ${pc.dim(SYMBOLS.present)} ${source}`)
  },

  /** Print a red cross warning when a source install fails. */
  warnSourceFailed(source: string, reason: string): void {
    console.error(`  ${pc.red(SYMBOLS.error)} ${source}: ${reason}`)
  },

  /**
   * Print the exact command being run.
   * Plumbing for --verbose in Phase 2 — always printed for now.
   */
  printCommand(cmd: string): void {
    console.log(`  ${pc.dim(cmd)}`)
  },

  /**
   * Print a command prefixed with "(dry run)" indicator.
   * Used when --dry-run is active — no subprocess is spawned.
   */
  printDryRunCommand(cmd: string): void {
    console.log(`  ${pc.dim('(dry run)')} ${pc.dim(cmd)}`)
  },

  /**
   * Print the final summary line after a sync.
   * Format: "X installed, Y removed, Z unchanged" (locked output format).
   */
  printSummary(installed: number, removed: number, unchanged: number): void {
    console.log(`\n${pc.bold(`${installed} installed, ${removed} removed, ${unchanged} unchanged`)}`)
  },

  /**
   * Print the dry-run summary line.
   * Format: "X to install, Y to remove, Z unchanged (dry run)" (locked output format).
   */
  printDryRunSummary(toInstall: number, toRemove: number, unchanged: number): void {
    console.log(
      `\n${pc.bold(`${toInstall} to install, ${toRemove} to remove, ${unchanged} unchanged`)} ${pc.dim('(dry run)')}`,
    )
  },

  /**
   * Print a user-facing error with remediation hint.
   * Outputs "Error: <message>" then "Hint: <hint>" on separate lines to stderr.
   */
  printError(message: string, hint: string): void {
    console.error(`${pc.red('Error:')} ${message}`)
    console.error(`${pc.dim('Hint:')} ${hint}`)
  },

  /**
   * Warn when lock file schema version doesn't match EXPECTED_LOCK_VERSION.
   * Falls back to empty state — does NOT abort.
   */
  warnLockVersionMismatch(actual: number, expected: number): void {
    console.error(
      pc.yellow(
        `Warning: lock file version ${actual} does not match expected ${expected}. Treating as empty state.`,
      ),
    )
  },

  /**
   * Warn when lock file JSON is corrupt/unparseable.
   * Falls back to empty state — does NOT abort.
   */
  warnLockFileCorrupt(path: string): void {
    console.error(
      pc.yellow(`Warning: lock file at ${path} is corrupt or unreadable. Treating as empty state.`),
    )
  },

  /** Print orphaned skills list with yellow header */
  printOrphans(skillNames: string[]): void {
    console.log(`\n${pc.yellow('Orphaned skills (installed but not in manifest):')}`)
    for (const name of skillNames) {
      console.log(`  ${pc.red(SYMBOLS.error)} ${name}`)
    }
  },

  /** Print removing indicator */
  printRemoving(skillName: string): void {
    console.log(`  ${pc.cyan(SYMBOLS.installing)} removing ${skillName}`)
  },

  /** Print removed confirmation */
  printRemoved(skillName: string): void {
    console.log(`  ${pc.dim(SYMBOLS.present)} removed ${skillName}`)
  },

  /** Warn when a skill removal fails */
  warnRemoveFailed(skillName: string, reason: string): void {
    console.error(`  ${pc.red(SYMBOLS.error)} remove ${skillName}: ${reason}`)
  },

  /** Warn when orphan removal is being skipped */
  warnSkippingOrphans(reason: string): void {
    console.log(`  ${pc.dim(SYMBOLS.pending)} skipping orphan removal: ${reason}`)
  },

  /** Print pre-sync diff preview showing what will happen */
  printPreSyncPreview(diff: { toAdd: string[]; toRemove: string[]; unchanged: string[] }): void {
    if (diff.toAdd.length > 0) {
      console.log(`\n${pc.cyan('To install:')}`)
      for (const source of diff.toAdd) {
        console.log(`  ${pc.cyan(SYMBOLS.installing)} ${source}`)
      }
    }
    if (diff.toRemove.length > 0) {
      console.log(`\n${pc.yellow('To remove:')}`)
      for (const name of diff.toRemove) {
        console.log(`  ${pc.red(SYMBOLS.error)} ${name}`)
      }
    }
  },
}
