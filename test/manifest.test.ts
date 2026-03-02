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
// parseManifest — JSON parsing and validation
// ---------------------------------------------------------------------------

describe('parseManifest', () => {
  it('parses valid JSON and returns Manifest with correct sources', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'valid.skills.json'), 'utf-8')
    const manifest = parseManifest(content, 'valid.skills.json')
    expect(manifest.sources).toEqual(['anthropics/claude-code', 'vercel-labs/skills'])
  })

  it('parses empty repos array as valid Manifest', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'empty-repos.skills.json'), 'utf-8')
    const manifest = parseManifest(content, 'empty-repos.skills.json')
    expect(manifest.sources).toEqual([])
  })

  it('throws ManifestParseError on invalid JSON', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'invalid.skills.json'), 'utf-8')
    expect(() => parseManifest(content, 'invalid.skills.json')).toThrow(ManifestParseError)
    try {
      parseManifest(content, 'invalid.skills.json')
    } catch (err) {
      expect(err).toBeInstanceOf(ManifestParseError)
      const msg = (err as ManifestParseError).message
      expect(msg).toMatch(/invalid\.skills\.json/)
      // Ensure the error carries a hint
      expect((err as ManifestParseError).hint).toBeTruthy()
    }
  })

  it('throws ManifestValidationError on duplicate sources', async () => {
    const content = await fs.readFile(path.join(FIXTURES, 'duplicate-repos.skills.json'), 'utf-8')
    expect(() => parseManifest(content, 'duplicate-repos.skills.json')).toThrow(ManifestValidationError)
    try {
      parseManifest(content, 'duplicate-repos.skills.json')
    } catch (err) {
      expect((err as ManifestValidationError).message).toMatch(/duplicate/i)
    }
  })

  it('throws ManifestValidationError when "skills" object is missing', () => {
    const content = '{"name": "test"}'
    expect(() => parseManifest(content, 'test.json')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when repos key is missing', () => {
    const content = '{"skills": {"name": "test"}}'
    expect(() => parseManifest(content, 'test.json')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when repos is not an array', () => {
    const content = '{"skills": {"repos": "owner/repo"}}'
    expect(() => parseManifest(content, 'test.json')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when a source is an empty string', () => {
    const content = '{"skills": {"repos": ["owner/repo", ""]}}'
    expect(() => parseManifest(content, 'test.json')).toThrow(ManifestValidationError)
  })

  it('throws ManifestValidationError when a source is not a string', () => {
    const content = '{"skills": {"repos": [42]}}'
    expect(() => parseManifest(content, 'test.json')).toThrow(ManifestValidationError)
  })

  it('trims whitespace from sources but does not normalize', () => {
    const content = '{"skills": {"repos": ["  owner/repo  "]}}'
    const manifest = parseManifest(content, 'test.json')
    expect(manifest.sources).toEqual(['owner/repo'])
  })
})

// ---------------------------------------------------------------------------
// resolveManifestPath — config discovery order
// ---------------------------------------------------------------------------

describe('resolveManifestPath', () => {
  it('returns --config path when provided (highest priority)', () => {
    const result = resolveManifestPath('/custom/path/manifest.json', 'local')
    expect(result).toBe('/custom/path/manifest.json')
  })

  it('returns --config path even when scope is global', () => {
    const result = resolveManifestPath('/custom/path/manifest.json', 'global')
    expect(result).toBe('/custom/path/manifest.json')
  })

  it('returns CWD/.skills.json when no configPath and scope is local', () => {
    const result = resolveManifestPath(undefined, 'local')
    expect(result).toBe(path.join(process.cwd(), '.skills.json'))
  })

  it('returns ~/.config/skills-sync/skills.json when no configPath and scope is global', () => {
    const result = resolveManifestPath(undefined, 'global')
    expect(result).toBe(path.join(os.homedir(), '.config', 'skills-sync', 'skills.json'))
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
    const manifestPath = path.join(tempDir, 'test.skills.json')
    await fs.writeFile(manifestPath, '{"skills": {"repos": ["owner/repo"]}}\n', 'utf-8')
    const manifest = await readManifest(manifestPath, 'local')
    expect(manifest.sources).toEqual(['owner/repo'])
  })

  it('throws ManifestNotFoundError with --init hint on missing file', async () => {
    const missingPath = path.join(tempDir, 'nonexistent.skills.json')
    await expect(readManifest(missingPath, 'local')).rejects.toThrow(ManifestNotFoundError)
    try {
      await readManifest(missingPath, 'local')
    } catch (err) {
      expect(err).toBeInstanceOf(ManifestNotFoundError)
      expect((err as ManifestNotFoundError).hint).toMatch(/--init/)
    }
  })

  it('ManifestNotFoundError message includes the file path', async () => {
    const missingPath = path.join(tempDir, 'nonexistent.skills.json')
    try {
      await readManifest(missingPath, 'local')
    } catch (err) {
      expect((err as ManifestNotFoundError).message).toContain(missingPath)
    }
  })
})
