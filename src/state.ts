import { readFile, readdir } from 'node:fs/promises'
import { join } from 'node:path'
import { homedir } from 'node:os'
import type { InstalledState } from './types.js'

/** Lock file version this tool understands. Mismatch triggers fallback to empty state. */
const EXPECTED_LOCK_VERSION = 3

/** Default path to the global skills lock file managed by Vercel's skills CLI. */
const DEFAULT_LOCK_PATH = join(homedir(), '.agents', '.skill-lock.json')

/**
 * Normalize a source string for stable comparison and map keying.
 * Exported for shared use in diff.ts.
 */
export function normalizeSource(source: string): string {
  return source.trim().toLowerCase().replace(/\/+$/, '')
}

/** Lock file structure (read-only — we never write to this file). */
interface LockFile {
  version: number
  skills: Record<string, { source: string }>
}

/**
 * Read installed state from the global lock file at ~/.agents/.skill-lock.json.
 *
 * Falls back to empty state (without throwing) when:
 *   - File does not exist
 *   - JSON is corrupt or cannot be parsed
 *   - Version does not match EXPECTED_LOCK_VERSION (logs a warning)
 *
 * @param lockPath Override for testing; defaults to ~/.agents/.skill-lock.json
 */
export async function readGlobalState(
  lockPath: string = DEFAULT_LOCK_PATH
): Promise<InstalledState> {
  const empty: InstalledState = { bySource: new Map() }

  let raw: string
  try {
    raw = await readFile(lockPath, 'utf-8')
  } catch {
    // Missing file is expected on first run — not an error
    return empty
  }

  let lock: LockFile
  try {
    lock = JSON.parse(raw) as LockFile
  } catch (err) {
    console.error(
      `[skills-sync] Failed to parse lock file at ${lockPath}: ${err instanceof Error ? err.message : String(err)}`
    )
    return empty
  }

  if (lock.version !== EXPECTED_LOCK_VERSION) {
    console.error(
      `[skills-sync] Lock file version mismatch: expected ${EXPECTED_LOCK_VERSION}, got ${lock.version}. ` +
        `Falling back to empty state. Please update skills-sync or re-run \`npx skills\`.`
    )
    return empty
  }

  const bySource = new Map<string, string[]>()

  for (const [skillName, skillData] of Object.entries(lock.skills ?? {})) {
    const key = normalizeSource(skillData.source)
    const group = bySource.get(key) ?? []
    group.push(skillName)
    bySource.set(key, group)
  }

  return { bySource }
}

/** Subdirectories to scan for locally installed skills. */
const LOCAL_SKILL_DIRS = ['.agents/skills', '.claude/skills'] as const

/**
 * Read installed state by scanning the local filesystem.
 *
 * Because there is no per-skill source mapping without `.skills-sync-sources.json`
 * (Phase 2 gap — upstream issue #337), all locally detected skills are grouped
 * under the synthetic source key `"__local__"`.
 *
 * Falls back to empty state (without throwing) when directories are missing.
 *
 * @param basePath Override for testing; defaults to process.cwd()
 */
export async function readLocalState(
  basePath: string = process.cwd()
): Promise<InstalledState> {
  const skillNames: string[] = []

  for (const relDir of LOCAL_SKILL_DIRS) {
    const dir = join(basePath, relDir)
    let entries: string[]
    try {
      const dirents = await readdir(dir, { withFileTypes: true })
      entries = dirents.filter((d) => d.isDirectory()).map((d) => d.name)
    } catch {
      // Directory doesn't exist — silently skip
      continue
    }
    skillNames.push(...entries)
  }

  if (skillNames.length === 0) {
    return { bySource: new Map() }
  }

  return {
    bySource: new Map([['__local__', skillNames]]),
  }
}
