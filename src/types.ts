/**
 * Core shared types for skills-sync.
 * All pipeline stages pass these typed data structures forward.
 */

/** Parsed representation of .skills.toml manifest */
export interface Manifest {
  sources: string[]
}

/**
 * Actual installed state read from lock file (global) or filesystem (local).
 * key: normalized source string, value: list of skill names from that source
 */
export interface InstalledState {
  bySource: Map<string, string[]>
}

/**
 * Diff between desired state (Manifest) and actual state (InstalledState).
 * Phase 1: toRemove is always empty (orphan detection is Phase 2).
 */
export interface Diff {
  toAdd: string[]      // sources to install (in manifest, not installed)
  toRemove: string[]   // skill names to remove (orphans) — Phase 2
  unchanged: string[]  // sources already present
}

/** Options passed from CLI flags to pipeline stages */
export interface SyncOptions {
  scope: 'global' | 'local'
  dryRun: boolean
  configPath?: string
  yes?: boolean       // --yes: skip prompts, pass -y to subcommands
  verbose?: boolean   // --verbose: show npx commands being run
  agent?: string      // --agent: passed to npx skills add/remove
}

/** Summary result of a sync operation */
export interface SyncResult {
  installed: number
  failed: number
  unchanged: number
  removed: number
}

/** Schema for .skills-sync-sources.json — skills-sync-owned source attribution file */
export interface SourcesFile {
  version: 1
  /** normalized source → skill names installed from that source */
  sources: Record<string, string[]>
}
