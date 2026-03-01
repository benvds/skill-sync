import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import * as fs from 'node:fs/promises'
import * as path from 'node:path'
import * as os from 'node:os'
import { generateManifestTemplate, writeManifestTemplate } from '../src/init.js'

// ---------------------------------------------------------------------------
// generateManifestTemplate
// ---------------------------------------------------------------------------

describe('generateManifestTemplate', () => {
  it('returns a string', () => {
    expect(typeof generateManifestTemplate()).toBe('string')
  })

  it('contains [skills] section', () => {
    expect(generateManifestTemplate()).toContain('[skills]')
  })

  it('contains repos array', () => {
    expect(generateManifestTemplate()).toContain('repos = [')
  })

  it('contains GitHub shorthand example as a comment', () => {
    expect(generateManifestTemplate()).toMatch(/owner\/repo/)
  })

  it('contains full URL example as a comment', () => {
    expect(generateManifestTemplate()).toMatch(/https:\/\/github\.com/)
  })

  it('contains local path example as a comment', () => {
    expect(generateManifestTemplate()).toMatch(/\.\//)
  })

  it('contains usage instructions for npx skills-sync', () => {
    expect(generateManifestTemplate()).toContain('npx skills-sync')
  })

  it('contains --global usage hint', () => {
    expect(generateManifestTemplate()).toContain('--global')
  })

  it('contains --dry-run usage hint', () => {
    expect(generateManifestTemplate()).toContain('--dry-run')
  })
})

// ---------------------------------------------------------------------------
// writeManifestTemplate
// ---------------------------------------------------------------------------

describe('writeManifestTemplate', () => {
  let tempDir: string
  let originalCwd: string

  beforeEach(async () => {
    const rawTempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'skills-sync-init-test-'))
    // Resolve symlinks so that process.cwd() and tempDir match on macOS (/private/var vs /var)
    tempDir = await fs.realpath(rawTempDir)
    originalCwd = process.cwd()
    process.chdir(tempDir)
  })

  afterEach(async () => {
    process.chdir(originalCwd)
    await fs.rm(tempDir, { recursive: true, force: true })
  })

  it('creates .skills.toml in CWD for local scope', async () => {
    const writtenPath = await writeManifestTemplate('local')
    expect(writtenPath).toBe(path.join(tempDir, '.skills.toml'))
    const content = await fs.readFile(writtenPath, 'utf-8')
    expect(content).toContain('[skills]')
  })

  it('returns the path it wrote to', async () => {
    const writtenPath = await writeManifestTemplate('local')
    expect(writtenPath).toBe(path.join(tempDir, '.skills.toml'))
  })

  it('creates global config file for global scope', async () => {
    const globalDir = path.join(tempDir, 'config', 'skills-sync')
    // Override HOME to use temp dir
    const origHome = os.homedir()
    process.env.HOME = tempDir

    try {
      const writtenPath = await writeManifestTemplate('global')
      const expectedPath = path.join(tempDir, '.config', 'skills-sync', 'skills.toml')
      expect(writtenPath).toBe(expectedPath)
      const stat = await fs.stat(writtenPath)
      expect(stat.isFile()).toBe(true)
    } finally {
      process.env.HOME = origHome
      // Clean up global config created during test
      await fs.rm(path.join(tempDir, '.config'), { recursive: true, force: true })
    }
  })

  it('refuses to overwrite an existing file', async () => {
    // Write first time — should succeed
    await writeManifestTemplate('local')
    // Attempt to write again — should throw
    await expect(writeManifestTemplate('local')).rejects.toThrow()
  })

  it('error message mentions the existing file path when refusing to overwrite', async () => {
    await writeManifestTemplate('local')
    try {
      await writeManifestTemplate('local')
    } catch (err) {
      expect((err as Error).message).toContain('.skills.toml')
    }
  })
})
