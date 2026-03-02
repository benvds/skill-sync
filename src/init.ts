/**
 * --init command: generate and write a sample .skills.json template.
 */

import * as fs from 'node:fs/promises'
import * as path from 'node:path'
import * as os from 'node:os'

/**
 * Generate a .skills.json template.
 * Template includes an empty repos array ready for the user to populate.
 */
export function generateManifestTemplate(): string {
  return JSON.stringify(
    {
      skills: {
        repos: [],
      },
    },
    null,
    2,
  ) + '\n'
}

/**
 * Write the manifest template to disk.
 * - local scope: writes to CWD/.skills.json
 * - global scope: writes to ~/.config/skills-sync/skills.json (creates parent dirs)
 *
 * @throws Error if file already exists (refuses to overwrite)
 * @returns the absolute path written to
 */
export async function writeManifestTemplate(scope: 'global' | 'local'): Promise<string> {
  const outputPath = resolveOutputPath(scope)

  // Refuse to overwrite existing file
  try {
    await fs.access(outputPath)
    // If we get here, the file exists
    throw new Error(
      `File already exists: ${outputPath}. Remove it first if you want to regenerate.`,
    )
  } catch (err) {
    // If the error is from our own throw (file exists), re-throw it
    if (err instanceof Error && err.message.startsWith('File already exists:')) {
      throw err
    }
    // Otherwise, file does not exist — proceed
  }

  // Create parent directories if needed (important for global scope)
  await fs.mkdir(path.dirname(outputPath), { recursive: true })

  const template = generateManifestTemplate()
  await fs.writeFile(outputPath, template, 'utf-8')

  return outputPath
}

function resolveOutputPath(scope: 'global' | 'local'): string {
  if (scope === 'local') {
    return path.join(process.cwd(), '.skills.json')
  }
  return path.join(os.homedir(), '.config', 'skills-sync', 'skills.json')
}
