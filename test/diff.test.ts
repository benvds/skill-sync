import { describe, it, expect } from 'vitest'
import { computeDiff, normalizeSource } from '../src/diff.js'
import type { Manifest, InstalledState } from '../src/types.js'

// Helper to build InstalledState from a list of normalized source keys
function makeState(...sources: string[]): InstalledState {
  const bySource = new Map<string, string[]>()
  for (const src of sources) {
    bySource.set(src, ['some-skill'])
  }
  return { bySource }
}

describe('normalizeSource', () => {
  it('lowercases the source string', () => {
    expect(normalizeSource('Owner/Repo')).toBe('owner/repo')
  })

  it('trims leading and trailing whitespace', () => {
    expect(normalizeSource('  owner/repo  ')).toBe('owner/repo')
  })

  it('strips a trailing slash', () => {
    expect(normalizeSource('owner/repo/')).toBe('owner/repo')
  })

  it('strips multiple trailing slashes', () => {
    expect(normalizeSource('owner/repo//')).toBe('owner/repo')
  })

  it('handles combination of trim + lowercase + trailing slash', () => {
    expect(normalizeSource('  Owner/Repo/  ')).toBe('owner/repo')
  })
})

describe('computeDiff', () => {
  it('returns all manifest sources in toAdd when state is empty', () => {
    const manifest: Manifest = { sources: ['a/b', 'c/d'] }
    const state = makeState()

    const diff = computeDiff(manifest, state)

    expect(diff.toAdd).toEqual(expect.arrayContaining(['a/b', 'c/d']))
    expect(diff.toAdd).toHaveLength(2)
    expect(diff.unchanged).toHaveLength(0)
    expect(diff.toRemove).toHaveLength(0)
  })

  it('returns all manifest sources in unchanged when all are installed', () => {
    const manifest: Manifest = { sources: ['a/b'] }
    const state = makeState('a/b')

    const diff = computeDiff(manifest, state)

    expect(diff.toAdd).toHaveLength(0)
    expect(diff.unchanged).toContain('a/b')
    expect(diff.toRemove).toHaveLength(0)
  })

  it('correctly splits between toAdd and unchanged for mixed state', () => {
    const manifest: Manifest = { sources: ['a/b', 'c/d', 'e/f'] }
    const state = makeState('a/b', 'e/f')

    const diff = computeDiff(manifest, state)

    expect(diff.toAdd).toContain('c/d')
    expect(diff.toAdd).toHaveLength(1)
    expect(diff.unchanged).toContain('a/b')
    expect(diff.unchanged).toContain('e/f')
    expect(diff.unchanged).toHaveLength(2)
    expect(diff.toRemove).toHaveLength(0)
  })

  it('returns empty result for empty manifest regardless of state', () => {
    const manifest: Manifest = { sources: [] }
    const state = makeState('a/b', 'c/d')

    const diff = computeDiff(manifest, state)

    expect(diff.toAdd).toHaveLength(0)
    expect(diff.unchanged).toHaveLength(0)
    expect(diff.toRemove).toHaveLength(0)
  })

  it('performs case-insensitive source comparison', () => {
    const manifest: Manifest = { sources: ['Owner/Repo'] }
    const state = makeState('owner/repo')

    const diff = computeDiff(manifest, state)

    expect(diff.toAdd).toHaveLength(0)
    expect(diff.unchanged).toContain('Owner/Repo')
  })

  it('ignores trailing slashes when comparing sources', () => {
    const manifest: Manifest = { sources: ['owner/repo/'] }
    const state = makeState('owner/repo')

    const diff = computeDiff(manifest, state)

    expect(diff.toAdd).toHaveLength(0)
    expect(diff.unchanged).toContain('owner/repo/')
  })

  it('preserves original source strings in toAdd (not normalized)', () => {
    const manifest: Manifest = { sources: ['Owner/Repo'] }
    const state = makeState()

    const diff = computeDiff(manifest, state)

    // Output must retain the original casing — normalization is for comparison only
    expect(diff.toAdd).toContain('Owner/Repo')
    expect(diff.toAdd).not.toContain('owner/repo')
  })

  it('preserves original source strings in unchanged (not normalized)', () => {
    const manifest: Manifest = { sources: ['Owner/Repo/'] }
    const state = makeState('owner/repo')

    const diff = computeDiff(manifest, state)

    // Output must retain the original form from the manifest
    expect(diff.unchanged).toContain('Owner/Repo/')
    expect(diff.unchanged).not.toContain('owner/repo')
  })

  it('toRemove is always empty in Phase 1', () => {
    const manifest: Manifest = { sources: [] }
    // Even with state having sources, toRemove must stay empty
    const state = makeState('a/b', 'c/d')

    const diff = computeDiff(manifest, state)

    expect(diff.toRemove).toHaveLength(0)
  })
})
