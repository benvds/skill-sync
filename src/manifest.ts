/**
 * Manifest reading, discovery, and parsing.
 *
 * Config discovery order (locked):
 * 1. --config <path> flag (always wins)
 * 2. CWD/.skills.toml (local scope)
 * 3. ~/.config/skills-sync/skills.toml (global scope)
 */

import * as fs from 'node:fs/promises'
import * as path from 'node:path'
import * as os from 'node:os'
import { parse, TomlError } from 'smol-toml'
import type { Manifest } from './types.js'
import {
  ManifestNotFoundError,
  ManifestParseError,
  ManifestValidationError,
} from './errors.js'

/**
 * Resolve the manifest path based on --config flag and scope.
 * Priority: --config > CWD/.skills.toml (local) > ~/.config/skills-sync/skills.toml (global)
 */
export function resolveManifestPath(
  configPath: string | undefined,
  scope: 'global' | 'local',
): string {
  if (configPath) return configPath

  if (scope === 'local') {
    return path.join(process.cwd(), '.skills.toml')
  }

  // global scope
  return path.join(os.homedir(), '.config', 'skills-sync', 'skills.toml')
}

/**
 * Read and parse the manifest at the given configPath (or discovered path).
 * Throws ManifestNotFoundError, ManifestParseError, or ManifestValidationError on failure.
 */
export async function readManifest(
  configPath: string | undefined,
  scope: 'global' | 'local',
): Promise<Manifest> {
  const resolvedPath = resolveManifestPath(configPath, scope)
  const content = await readManifestFile(resolvedPath)
  return parseManifest(content, resolvedPath)
}

async function readManifestFile(filePath: string): Promise<string> {
  try {
    return await fs.readFile(filePath, 'utf-8')
  } catch {
    throw new ManifestNotFoundError(filePath)
  }
}

/**
 * Parse TOML content into a Manifest.
 * Exposed for unit testing — not intended as a public API.
 *
 * @throws ManifestParseError   on TOML syntax errors (includes line/column from smol-toml)
 * @throws ManifestValidationError on structural problems (missing [skills], wrong types, duplicates)
 */
export function parseManifest(content: string, filePath: string): Manifest {
  let doc: unknown
  try {
    doc = parse(content)
  } catch (err) {
    if (err instanceof TomlError) {
      // smol-toml error messages include "at line N, column M"
      throw new ManifestParseError(filePath, err)
    }
    throw err
  }

  return validateManifest(doc, filePath)
}

function validateManifest(doc: unknown, filePath: string): Manifest {
  if (!isRecord(doc)) {
    throw new ManifestValidationError(filePath, 'top-level document must be a table')
  }

  if (!('skills' in doc)) {
    throw new ManifestValidationError(filePath, "missing [skills] table")
  }

  const skills = (doc as Record<string, unknown>).skills
  if (!isRecord(skills)) {
    throw new ManifestValidationError(filePath, "[skills] must be a table")
  }

  if (!('repos' in skills)) {
    throw new ManifestValidationError(filePath, "missing 'repos' key under [skills]")
  }

  const repos = (skills as Record<string, unknown>).repos
  if (!Array.isArray(repos)) {
    throw new ManifestValidationError(filePath, "'repos' must be an array")
  }

  // Validate each entry is a non-empty string
  const sources: string[] = []
  for (let i = 0; i < repos.length; i++) {
    const entry = repos[i]
    if (typeof entry !== 'string') {
      throw new ManifestValidationError(
        filePath,
        `repos[${i}] must be a string, got ${typeof entry}`,
      )
    }
    const trimmed = entry.trim()
    if (trimmed === '') {
      throw new ManifestValidationError(
        filePath,
        `repos[${i}] must not be an empty string`,
      )
    }
    sources.push(trimmed)
  }

  // Validate no duplicates (case-sensitive — normalization is diff's job)
  const seen = new Set<string>()
  for (const source of sources) {
    if (seen.has(source)) {
      throw new ManifestValidationError(
        filePath,
        `duplicate source: "${source}"`,
      )
    }
    seen.add(source)
  }

  return { sources }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
