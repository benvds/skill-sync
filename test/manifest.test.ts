import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import * as fs from 'node:fs/promises'
import * as path from 'node:path'
import * as os from 'node:os'
import { readManifest, resolveManifestPath, parseManifest } from '../src/manifest.js'
import {
  ManifestNotFoundError,
  ManifestParseError,
  ManifestValidationError,
} from '../src/errors.js'

const FIXTURES = path.join(import.meta.dirname, 'fixtures')

// ---------------------------------------------------------------------------
// parseManifest — TOML parsing and validation
// ---------------------------------------------------------------------------

describe('parseManifest', () => {
  it('parses valid TOML and returns Manifest with correct sources', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'valid.skills.toml'), 'utf-8')
    const manifest = parseManifest(content, 'valid.skills.toml')
    expect(manifest.sources).toEqual(['anthropics/claude-code', 'vercel-labs/skills'])
  })

  it('parses empty repos array as valid Manifest', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'empty-repos.skills.toml'), 'utf-8')
    const manifest = parseManifest(content, 'empty-repos.skills.toml')
    expect(manifest.sources).toEqual([])
  })

  it('throws ManifestParseError with line/column info on invalid TOML', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'invalid.skills.toml'), 'utf-8')
    expect(() => parseManifest(content, 'invalid.skills.toml')).toThrow(ManifestParseError)
    try {
      parseManifest(content, 'invalid.skills.toml')
    } catch (err) {
      expect(err).toBeInstanceOf(ManifestParseError)
      // smol-toml includes line/column in its error message
      const msg = (err as ManifestParseError).message
      expect(msg).toMatch(/invalid\.skills\.toml/)
      // Ensure the error carries a hint
      expect((err as ManifestParseError).hint).toBeTruthy()
    }
  })

  it('throws ManifestValidationError on duplicate sources', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'duplicate-repos.skills.toml'), 'utf-8')
    expect(() => parseManifest(content, 'duplicate-repos.skills.toml')).toThrow(ManifestValidationError)
    try {
      parseManifest(content, 'duplicate-repos.skills.toml')
    } catch (err) {
      expect((err as ManifestValidationError).message).toMatch(/duplicate/i)
    }
  })

  it('throws ManifestValidationError when [skills] table is missing', () => {
    const content = 'name = "test"'
    expect(() => parseManifest(content, 'test.toml')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when repos key is missing', () => {
    const content = '[skills]\nname = "test"'
    expect(() => parseManifest(content, 'test.toml')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when repos is not an array', () => {
    const content = '[skills]\nrepos = "owner/repo"'
    expect(() => parseManifest(content, 'test.toml')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when a source is an empty string', () => {
    const content = '[skills]\nrepos = ["owner/repo", ""]'
    expect(() => parseManifest(content, 'test.toml')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when a source is not a string', () => {
    const content = '[skills]\nrepos = [42]'
    expect(() => parseManifest(content, 'test.toml')).toThrow(ManifestValidationError)
  })

  it('trims whitespace from sources but does not normalize', () => {
    const content = '[skills]\nrepos = ["  owner/repo  "]'
    const manifest = parseManifest(content, 'test.toml')
    expect(manifest.sources).toEqual(['owner/repo'])
  })
})

// ---------------------------------------------------------------------------
// resolveManifestPath — config discovery order
// ---------------------------------------------------------------------------

describe('resolveManifestPath', () => {
  it('returns --config path when provided (highest priority)', () => {
    const result = resolveManifestPath('/custom/path/manifest.toml', 'local')
    expect(result).toBe('/custom/path/manifest.toml')
  })

  it('returns --config path even when scope is global', () => {
    const result = resolveManifestPath('/custom/path/manifest.toml', 'global')
    expect(result).toBe('/custom/path/manifest.toml')
  })

  it('returns CWD/.skills.toml when no configPath and scope is local', () => {
    const result = resolveManifestPath(undefined, 'local')
    expect(result).toBe(path.join(process.cwd(), '.skills.toml'))
  })

  it('returns ~/.config/skills-sync/skills.toml when no configPath and scope is global', () => {
    const result = resolveManifestPath(undefined, 'global')
    expect(result).toBe(path.join(os.homedir(), '.config', 'skills-sync', 'skills.toml'))
  })
})

// ---------------------------------------------------------------------------
// readManifest — file reading and full pipeline
// ---------------------------------------------------------------------------

describe('readManifest', () => {
  let tempDir: string

  beforeEach(async () => {
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'skills-sync-test-'))
  })

  afterEach(async () => {
    await fs.rm(tempDir, { recursive: true, force: true })
  })

  it('reads valid manifest file and returns Manifest', async () => {
    const manifestPath = path.join(tempDir, 'test.skills.toml')
    await fs.writeFile(manifestPath, '[skills]\nrepos = ["owner/repo"]\n', 'utf-8')
    const manifest = await readManifest(manifestPath, 'local')
    expect(manifest.sources).toEqual(['owner/repo'])
  })

  it('throws ManifestNotFoundError with --init hint on missing file', async () => {
    const missingPath = path.join(tempDir, 'nonexistent.skills.toml')
    await expect(readManifest(missingPath, 'local')).rejects.toThrow(ManifestNotFoundError)
    try {
      await readManifest(missingPath, 'local')
    } catch (err) {
      expect(err).toBeInstanceOf(ManifestNotFoundError)
      expect((err as ManifestNotFoundError).hint).toMatch(/--init/)
    }
  })

  it('ManifestNotFoundError message includes the file path', async () => {
    const missingPath = path.join(tempDir, 'nonexistent.skills.toml')
    try {
      await readManifest(missingPath, 'local')
    } catch (err) {
      expect((err as ManifestNotFoundError).message).toContain(missingPath)
    }
  })
})
