/**
 * Tests for --check flag — CI gate behavior.
 * Uses the built CLI binary for integration-level testing.
 */

import { describe, test, expect } from 'vitest'
import { execaNode } from 'execa'
import { resolve } from 'path'

const CLI = resolve(import.meta.dirname, '../dist/skills-sync.js')

describe('--check flag', () => {
  test('--check and --dry-run are mutually exclusive', async () => {
    const result = await execaNode(CLI, ['--check', '--dry-run'], { reject: false })
    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain('mutually exclusive')
  })

  test('--check exits non-zero when no manifest exists', async () => {
    // When no .skills.toml exists, readManifest throws — check mode still exits non-zero
    const result = await execaNode(CLI, ['--check'], {
      reject: false,
      cwd: import.meta.dirname, // test dir has no .skills.toml
    })
    expect(result.exitCode).toBe(1)
  })

  test('--check appears in help output', async () => {
    const result = await execaNode(CLI, ['--help'])
    expect(result.stdout).toContain('--check')
    expect(result.stdout).toContain('CI gate')
  })
})
