import type { Manifest, InstalledState, Diff } from './types.js'

/**
 * Normalize a source string for stable comparison.
 * Trims whitespace, lowercases, removes trailing slashes.
 *
 * Also exported from state.ts — keep implementations in sync.
 */
export function normalizeSource(source: string): string {
  return source.trim().toLowerCase().replace(/\/+$/, '')
}

/**
 * Compute the diff between desired state (manifest) and actual state (installed).
 *
 * Pure function — no I/O, no side effects.
 *
 * Source comparison is case-insensitive and trailing-slash-agnostic.
 * Output arrays contain the ORIGINAL source strings from the manifest (not normalized),
 * so downstream executor stages receive the exact strings the user wrote.
 *
 * Phase 1: toRemove is always empty. Orphan detection is deferred to Phase 2.
 */
export function computeDiff(manifest: Manifest, state: InstalledState): Diff {
  // Build a set of normalized keys from installed state for O(1) lookup
  const installedKeys = new Set<string>()
  for (const key of state.bySource.keys()) {
    installedKeys.add(normalizeSource(key))
  }

  const manifestKeys = new Set(manifest.sources.map(normalizeSource))

  const toAdd: string[] = []
  const unchanged: string[] = []

  for (const source of manifest.sources) {
    const normalized = normalizeSource(source)
    if (installedKeys.has(normalized)) {
      unchanged.push(source)
    } else {
      toAdd.push(source)
    }
  }

  // Orphan detection: skill names from sources not in manifest
  // Skip if state uses __local__ (no source attribution available)
  const toRemove: string[] = []
  if (!state.bySource.has('__local__')) {
    for (const [source, skillNames] of state.bySource) {
      if (!manifestKeys.has(normalizeSource(source))) {
        toRemove.push(...skillNames)
      }
    }
  }

  return { toAdd, unchanged, toRemove }
}
