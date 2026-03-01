import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { readGlobalState, readLocalState } from '../src/state.js'
import { SOURCES_FILENAME } from '../src/sources.js'

describe('readGlobalState', () => {
  const FIXTURE_DIR = new URL('./fixtures', import.meta.url).pathname

  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('returns correct bySource Map from valid lock file', async () => {
    const lockPath = join(FIXTURE_DIR, 'skill-lock.json')
    const state = await readGlobalState(lockPath)

    expect(state.bySource.size).toBe(2)
    expect(state.bySource.has('acme-org/dev-skills')).toBe(true)
    expect(state.bySource.has('owner/security-tools')).toBe(true)
  })

  it('groups skills from the same source together', async () => {
    const lockPath = join(FIXTURE_DIR, 'skill-lock.json')
    const state = await readGlobalState(lockPath)

    const devSkills = state.bySource.get('acme-org/dev-skills')
    expect(devSkills).toBeDefined()
    expect(devSkills).toHaveLength(2)
    expect(devSkills).toContain('git-helpers')
    expect(devSkills).toContain('code-review')
  })

  it('normalizes source keys to lowercase', async () => {
    const lockPath = join(FIXTURE_DIR, 'skill-lock.json')
    const state = await readGlobalState(lockPath)

    // "Owner/Security-Tools" should be normalized to "owner/security-tools"
    expect(state.bySource.has('owner/security-tools')).toBe(true)
    expect(state.bySource.has('Owner/Security-Tools')).toBe(false)
  })

  it('returns empty state for missing lock file (no error thrown)', async () => {
    const state = await readGlobalState('/nonexistent/path/.skill-lock.json')

    expect(state.bySource.size).toBe(0)
    expect(console.error).not.toHaveBeenCalled()
  })

  it('returns empty state and warns on corrupt JSON', async () => {
    const lockPath = join(FIXTURE_DIR, 'skill-lock-corrupt.json')
    const state = await readGlobalState(lockPath)

    expect(state.bySource.size).toBe(0)
    expect(console.error).toHaveBeenCalled()
  })

  it('returns empty state and warns on version mismatch', async () => {
    const lockPath = join(FIXTURE_DIR, 'skill-lock-v99.json')
    const state = await readGlobalState(lockPath)

    expect(state.bySource.size).toBe(0)
    expect(console.error).toHaveBeenCalled()
  })
})

describe('readLocalState', () => {
  let tmpDir: string

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'skills-sync-test-'))
  })

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true })
  })

  it('returns skill directories under __local__ key', async () => {
    // Create .agents/skills/ with two skill subdirectories
    const agentsSkillsDir = join(tmpDir, '.agents', 'skills')
    await mkdir(join(agentsSkillsDir, 'git-helpers'), { recursive: true })
    await mkdir(join(agentsSkillsDir, 'code-review'), { recursive: true })

    const state = await readLocalState(tmpDir)

    expect(state.bySource.has('__local__')).toBe(true)
    const local = state.bySource.get('__local__')!
    expect(local).toContain('git-helpers')
    expect(local).toContain('code-review')
  })

  it('scans both .agents/skills/ and .claude/skills/ directories', async () => {
    await mkdir(join(tmpDir, '.agents', 'skills', 'skill-a'), { recursive: true })
    await mkdir(join(tmpDir, '.claude', 'skills', 'skill-b'), { recursive: true })

    const state = await readLocalState(tmpDir)

    expect(state.bySource.has('__local__')).toBe(true)
    const local = state.bySource.get('__local__')!
    expect(local).toContain('skill-a')
    expect(local).toContain('skill-b')
  })

  it('returns empty state when no skill directories exist', async () => {
    const state = await readLocalState(tmpDir)

    expect(state.bySource.size).toBe(0)
  })

  it('returns empty state when base path does not exist', async () => {
    const state = await readLocalState('/nonexistent/path')

    expect(state.bySource.size).toBe(0)
  })

  it('returns bySource from .skills-sync-sources.json when present', async () => {
    const data = {
      version: 1,
      sources: { 'owner/repo': ['skill-a', 'skill-b'] },
    }
    await writeFile(join(tmpDir, SOURCES_FILENAME), JSON.stringify(data), 'utf-8')

    const state = await readLocalState(tmpDir)

    expect(state.bySource.has('owner/repo')).toBe(true)
    expect(state.bySource.get('owner/repo')).toEqual(['skill-a', 'skill-b'])
    expect(state.bySource.has('__local__')).toBe(false)
  })

  it('falls back to filesystem scan when .skills-sync-sources.json absent', async () => {
    await mkdir(join(tmpDir, '.agents', 'skills', 'some-skill'), { recursive: true })

    const state = await readLocalState(tmpDir)

    expect(state.bySource.has('__local__')).toBe(true)
  })

  it('falls back to filesystem scan when .skills-sync-sources.json has wrong version', async () => {
    const data = { version: 99, sources: {} }
    await writeFile(join(tmpDir, SOURCES_FILENAME), JSON.stringify(data), 'utf-8')
    await mkdir(join(tmpDir, '.agents', 'skills', 'some-skill'), { recursive: true })

    const state = await readLocalState(tmpDir)

    expect(state.bySource.has('__local__')).toBe(true)
  })

  it('returns multiple sources from .skills-sync-sources.json', async () => {
    const data = {
      version: 1,
      sources: {
        'owner/repo-a': ['skill-x'],
        'owner/repo-b': ['skill-y', 'skill-z'],
      },
    }
    await writeFile(join(tmpDir, SOURCES_FILENAME), JSON.stringify(data), 'utf-8')

    const state = await readLocalState(tmpDir)

    expect(state.bySource.size).toBe(2)
    expect(state.bySource.get('owner/repo-a')).toEqual(['skill-x'])
    expect(state.bySource.get('owner/repo-b')).toEqual(['skill-y', 'skill-z'])
  })
})
