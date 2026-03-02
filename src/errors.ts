/**
 * Typed error classes with user-facing remediation hints.
 * Every user-facing error must be a subclass of SkillsSyncError.
 * Each error carries a `hint` string shown below the error message.
 *
 * Usage in bin/skills-sync.ts:
 *   if (err instanceof SkillsSyncError) {
 *     reporter.printError(err.message, err.hint)
 *     process.exit(1)
 *   }
 */

export abstract class SkillsSyncError extends Error {
  abstract readonly hint: string

  constructor(message: string) {
    super(message)
    this.name = this.constructor.name
    // Ensure correct prototype chain for instanceof checks
    Object.setPrototypeOf(this, new.target.prototype)
  }
}

/** Thrown when no manifest file is found at the resolved path */
export class ManifestNotFoundError extends SkillsSyncError {
  readonly hint = "Run 'npx skills-sync --init' to create one"

  constructor(filePath: string) {
    super(`No manifest found at: ${filePath}`)
  }
}

/** Thrown when the manifest file contains invalid JSON syntax */
export class ManifestParseError extends SkillsSyncError {
  readonly hint: string

  constructor(filePath: string, cause: unknown) {
    const msg = cause instanceof Error ? cause.message : String(cause)
    super(`Invalid JSON in ${filePath}: ${msg}`)
    this.hint = 'Check the error above and fix the JSON syntax'
  }
}

/** Thrown when the manifest file does not match the expected structure */
export class ManifestValidationError extends SkillsSyncError {
  readonly hint: string

  constructor(filePath: string, reason: string) {
    super(`Invalid manifest at ${filePath}: ${reason}`)
    this.hint = 'Check the manifest structure — it must have a "skills" object with a "repos" array'
  }
}

/** Thrown when `npx skills` is not available on the system */
export class SkillsUnavailableError extends SkillsSyncError {
  readonly hint = 'Install it with: npm install -g skills'

  constructor() {
    super("'npx skills' command is unavailable")
  }
}
