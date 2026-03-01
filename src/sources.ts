/**
 * Source attribution for local-scope skill tracking.
 *
 * Reads/writes .skills-sync-sources.json — a skills-sync-owned file that maps
 * source strings to the skill directory names they installed. This enables
 * orphan detection: if a source is removed from the manifest, we know which
 * skill directories to remove.
 *
 * Also provides detectInstalledSkills() for filesystem scanning of skill directories.
 */

import { readFile, writeFile, rename, readdir } from 'node:fs/promises'
import { join } from 'node:path'
import type { SourcesFile } from './types.js'
import { normalizeSource } from './state.js'

/** Filename for the source attribution file */
export const SOURCES_FILENAME = '.skills-sync-sources.json'

/** Schema version — mismatch triggers fallback to null */
const SOURCES_VERSION = 1

/** Subdirectories to scan for locally installed skills */
const LOCAL_SKILL_DIRS = ['.agents/skills', '.claude/skills'] as const

/**
 * Read the source attribution file from basePath.
 *
 * Returns the parsed SourcesFile when valid, or null when:
 *   - File does not exist (ENOENT)
 *   - JSON is corrupt (SyntaxError)
 *   - Version does not match SOURCES_VERSION
 *
 * @param basePath Override for testing; defaults to process.cwd()
 */
export async function readSourcesFile(
  basePath: string = process.cwd()
): Promise<SourcesFile | null> {
  const filePath = join(basePath, SOURCES_FILENAME)
  try {
    const raw = await readFile(filePath, 'utf-8')
    const parsed = JSON.parse(raw) as SourcesFile
    if (parsed.version !== SOURCES_VERSION) return null
    return parsed
  } catch {
    return null
  }
}

/**
 * Write the source attribution file atomically using tmp+rename pattern.
 *
 * Normalizes source keys for consistent storage (lowercase, trimmed, no trailing slash).
 * Only records sources that were passed in — caller controls what gets written.
 *
 * @param bySource Map of source → skill names to persist
 * @param basePath Override for testing; defaults to process.cwd()
 */
export async function writeSourcesFile(
  bySource: Map<string, string[]>,
  basePath: string = process.cwd()
): Promise<void> {
  const sources: Record<string, string[]> = {}
  for (const [source, names] of bySource) {
    sources[normalizeSource(source)] = names
  }
  const data: SourcesFile = { version: SOURCES_VERSION, sources }
  const filePath = join(basePath, SOURCES_FILENAME)
  const tmpPath = `${filePath}.tmp`
  await writeFile(tmpPath, JSON.stringify(data, null, 2), 'utf-8')
  await rename(tmpPath, filePath)
}

/**
 * Detect installed skill directory names by scanning .agents/skills/ and .claude/skills/.
 *
 * Returns deduplicated array of skill directory names (ignores files, only directories).
 *
 * @param basePath Override for testing; defaults to process.cwd()
 */
export async function detectInstalledSkills(
  basePath: string = process.cwd()
): Promise<string[]> {
  const names = new Set<string>()
  for (const relDir of LOCAL_SKILL_DIRS) {
    const dir = join(basePath, relDir)
    try {
      const entries = await readdir(dir, { withFileTypes: true })
      for (const entry of entries) {
        if (entry.isDirectory()) {
          names.add(entry.name)
        }
      }
    } catch {
      // Directory doesn't exist — silently skip
    }
  }
  return [...names]
}
