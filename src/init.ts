/**
 * --init command: generate and write a sample .skills.toml template.
 */

import * as fs from 'node:fs/promises'
import * as path from 'node:path'
import * as os from 'node:os'

/**
 * Generate a comprehensive .skills.toml template with documentation comments.
 * Template includes: usage instructions, all supported source formats, and empty repos array.
 */
export function generateManifestTemplate(): string {
  return `# ============================================================================
# .skills.toml - Declarative skill manifest
# ============================================================================
#
# This file declares which skills your project needs.
# Run \`npx skills-sync\` to install and sync all listed skills.
#
# USAGE:
#   npx skills-sync              Sync project skills (default)
#   npx skills-sync --global     Sync global skills
#   npx skills-sync --dry-run    Preview changes
#
# SOURCE FORMAT:
#   Each entry is passed to \`npx skills add\`, so any valid source works:
#
#   "owner/repo"                         GitHub shorthand
#   "https://github.com/owner/repo"      Full URL
#   "https://gitlab.com/group/repo"      GitLab
#   "./local/path"                       Local directory
#
# FINDING SKILLS:
#   npx skills find <query>              Search the skills registry
#   https://skills.sh                    Browse available skills
#
# ============================================================================

[skills]
repos = [
  # Add your skill sources here:
  # "owner/repo-name",
]
`
}

/**
 * Write the manifest template to disk.
 * - local scope: writes to CWD/.skills.toml
 * - global scope: writes to ~/.config/skills-sync/skills.toml (creates parent dirs)
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
    return path.join(process.cwd(), '.skills.toml')
  }
  return path.join(os.homedir(), '.config', 'skills-sync', 'skills.toml')
}
