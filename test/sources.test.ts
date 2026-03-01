import { describe, it, expect, afterEach, beforeEach } from 'vitest'
import { mkdtemp, rm, mkdir, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { readSourcesFile, writeSourcesFile, detectInstalledSkills, SOURCES_FILENAME } from '../src/sources.js'

describe('readSourcesFile', () => {
  let tmpDir: string

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'skills-sync-sources-test-'))
  })

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true })
  })

  it('returns parsed SourcesFile when valid file exists', async () => {
    const data = { version: 1, sources: { 'owner/repo': ['skill-a', 'skill-b'] } }
    await writeFile(join(tmpDir, SOURCES_FILENAME), JSON.stringify(data), 'utf-8')

    const result = await readSourcesFile(tmpDir)

    expect(result).not.toBeNull()
    expect(result!.version).toBe(1)
    expect(result!.sources['owner/repo']).toEqual(['skill-a', 'skill-b'])
  })

  it('returns null when file is missing', async () => {
    const result = await readSourcesFile(tmpDir)
    expect(result).toBeNull()
  })

  it('returns null when JSON is corrupt', async () => {
    await writeFile(join(tmpDir, SOURCES_FILENAME), 'not json', 'utf-8')

    const result = await readSourcesFile(tmpDir)
    expect(result).toBeNull()
  })

  it('returns null when version does not match', async () => {
    const data = { version: 99, sources: {} }
    await writeFile(join(tmpDir, SOURCES_FILENAME), JSON.stringify(data), 'utf-8')

    const result = await readSourcesFile(tmpDir)
    expect(result).toBeNull()
  })

  it('returns SourcesFile with empty sources when valid', async () => {
    const data = { version: 1, sources: {} }
    await writeFile(join(tmpDir, SOURCES_FILENAME), JSON.stringify(data), 'utf-8')

    const result = await readSourcesFile(tmpDir)

    expect(result).not.toBeNull()
    expect(result!.version).toBe(1)
    expect(Object.keys(result!.sources)).toHaveLength(0)
  })
})

describe('writeSourcesFile', () => {
  let tmpDir: string

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'skills-sync-sources-test-'))
  })

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true })
  })

  it('writes correct JSON structure', async () => {
    const bySource = new Map([['owner/repo', ['skill-a', 'skill-b']]])
    await writeSourcesFile(bySource, tmpDir)

    const result = await readSourcesFile(tmpDir)
    expect(result).not.toBeNull()
    expect(result!.version).toBe(1)
    expect(result!.sources['owner/repo']).toEqual(['skill-a', 'skill-b'])
  })

  it('normalizes source keys', async () => {
    const bySource = new Map([['Owner/Repo', ['skill-a']]])
    await writeSourcesFile(bySource, tmpDir)

    const result = await readSourcesFile(tmpDir)
    expect(result).not.toBeNull()
    // Normalized key should be lowercase
    expect(result!.sources['owner/repo']).toEqual(['skill-a'])
    expect(result!.sources['Owner/Repo']).toBeUndefined()
  })

  it('file is valid JSON after write', async () => {
    const bySource = new Map([['a/b', ['x']]])
    await writeSourcesFile(bySource, tmpDir)

    const { readFile } = await import('node:fs/promises')
    const raw = await readFile(join(tmpDir, SOURCES_FILENAME), 'utf-8')
    const parsed = JSON.parse(raw)
    expect(parsed.version).toBe(1)
    expect(parsed.sources).toBeDefined()
  })

  it('handles empty map', async () => {
    const bySource = new Map<string, string[]>()
    await writeSourcesFile(bySource, tmpDir)

    const result = await readSourcesFile(tmpDir)
    expect(result).not.toBeNull()
    expect(result!.version).toBe(1)
    expect(Object.keys(result!.sources)).toHaveLength(0)
  })
})

describe('detectInstalledSkills', () => {
  let tmpDir: string

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'skills-sync-sources-test-'))
  })

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true })
  })

  it('returns skill directories from .agents/skills/', async () => {
    await mkdir(join(tmpDir, '.agents', 'skills', 'git-helpers'), { recursive: true })
    await mkdir(join(tmpDir, '.agents', 'skills', 'code-review'), { recursive: true })

    const result = await detectInstalledSkills(tmpDir)

    expect(result).toContain('git-helpers')
    expect(result).toContain('code-review')
  })

  it('returns skill directories from .claude/skills/', async () => {
    await mkdir(join(tmpDir, '.claude', 'skills', 'my-skill'), { recursive: true })

    const result = await detectInstalledSkills(tmpDir)

    expect(result).toContain('my-skill')
  })

  it('returns combined results from both directories', async () => {
    await mkdir(join(tmpDir, '.agents', 'skills', 'skill-a'), { recursive: true })
    await mkdir(join(tmpDir, '.claude', 'skills', 'skill-b'), { recursive: true })

    const result = await detectInstalledSkills(tmpDir)

    expect(result).toContain('skill-a')
    expect(result).toContain('skill-b')
  })

  it('returns empty array when no directories exist', async () => {
    const result = await detectInstalledSkills(tmpDir)
    expect(result).toEqual([])
  })

  it('ignores files (only directories count)', async () => {
    await mkdir(join(tmpDir, '.agents', 'skills'), { recursive: true })
    await writeFile(join(tmpDir, '.agents', 'skills', 'not-a-skill.txt'), 'data')
    await mkdir(join(tmpDir, '.agents', 'skills', 'real-skill'), { recursive: true })

    const result = await detectInstalledSkills(tmpDir)

    expect(result).toContain('real-skill')
    expect(result).not.toContain('not-a-skill.txt')
  })

  it('deduplicates skills present in both directories', async () => {
    await mkdir(join(tmpDir, '.agents', 'skills', 'shared-skill'), { recursive: true })
    await mkdir(join(tmpDir, '.claude', 'skills', 'shared-skill'), { recursive: true })

    const result = await detectInstalledSkills(tmpDir)

    const count = result.filter(n => n === 'shared-skill').length
    expect(count).toBe(1)
  })
})
