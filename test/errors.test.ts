import { describe, it, expect } from 'vitest'
import {
  SkillsSyncError,
  ManifestNotFoundError,
  ManifestParseError,
  SkillsUnavailableError,
  ManifestValidationError,
} from '../src/errors.js'

describe('SkillsSyncError hierarchy', () => {
  describe('ManifestNotFoundError', () => {
    it('is instanceof SkillsSyncError', () => {
      const err = new ManifestNotFoundError('/path/to/.skills.toml')
      expect(err).toBeInstanceOf(SkillsSyncError)
    })

    it('is instanceof Error', () => {
      const err = new ManifestNotFoundError('/path/to/.skills.toml')
      expect(err).toBeInstanceOf(Error)
    })

    it('has correct message format', () => {
      const err = new ManifestNotFoundError('/path/to/.skills.toml')
      expect(err.message).toContain('/path/to/.skills.toml')
    })

    it('has non-empty hint string', () => {
      const err = new ManifestNotFoundError('/path/to/.skills.toml')
      expect(err.hint).toBeTruthy()
      expect(typeof err.hint).toBe('string')
    })

    it('hint mentions --init', () => {
      const err = new ManifestNotFoundError('/path/to/.skills.toml')
      expect(err.hint).toContain('--init')
    })
  })

  describe('ManifestParseError', () => {
    it('is instanceof SkillsSyncError', () => {
      const cause = new Error('Unexpected token at line 3, column 5')
      const err = new ManifestParseError('/path/to/.skills.toml', cause)
      expect(err).toBeInstanceOf(SkillsSyncError)
    })

    it('is instanceof Error', () => {
      const cause = new Error('Unexpected token at line 3, column 5')
      const err = new ManifestParseError('/path/to/.skills.toml', cause)
      expect(err).toBeInstanceOf(Error)
    })

    it('includes file path in message', () => {
      const cause = new Error('Unexpected token at line 3, column 5')
      const err = new ManifestParseError('/path/to/.skills.toml', cause)
      expect(err.message).toContain('/path/to/.skills.toml')
    })

    it('includes cause message in error message', () => {
      const cause = new Error('Unexpected token at line 3, column 5')
      const err = new ManifestParseError('/path/to/.skills.toml', cause)
      expect(err.message).toContain('Unexpected token at line 3, column 5')
    })

    it('handles non-Error cause', () => {
      const err = new ManifestParseError('/path/to/.skills.toml', 'string cause')
      expect(err.message).toContain('string cause')
    })

    it('has non-empty hint string', () => {
      const cause = new Error('syntax error')
      const err = new ManifestParseError('/path/to/.skills.toml', cause)
      expect(err.hint).toBeTruthy()
      expect(typeof err.hint).toBe('string')
    })

    it('hint mentions line/column', () => {
      const cause = new Error('syntax error')
      const err = new ManifestParseError('/path/to/.skills.toml', cause)
      expect(err.hint.toLowerCase()).toMatch(/line|column|syntax/)
    })
  })

  describe('SkillsUnavailableError', () => {
    it('is instanceof SkillsSyncError', () => {
      const err = new SkillsUnavailableError()
      expect(err).toBeInstanceOf(SkillsSyncError)
    })

    it('is instanceof Error', () => {
      const err = new SkillsUnavailableError()
      expect(err).toBeInstanceOf(Error)
    })

    it('has correct message', () => {
      const err = new SkillsUnavailableError()
      expect(err.message).toContain('skills')
    })

    it('has non-empty hint string', () => {
      const err = new SkillsUnavailableError()
      expect(err.hint).toBeTruthy()
      expect(typeof err.hint).toBe('string')
    })

    it('hint mentions install', () => {
      const err = new SkillsUnavailableError()
      expect(err.hint.toLowerCase()).toContain('install')
    })
  })

  describe('ManifestValidationError', () => {
    it('is instanceof SkillsSyncError', () => {
      const err = new ManifestValidationError('/path/to/.skills.toml', 'missing [skills] table')
      expect(err).toBeInstanceOf(SkillsSyncError)
    })

    it('is instanceof Error', () => {
      const err = new ManifestValidationError('/path/to/.skills.toml', 'missing [skills] table')
      expect(err).toBeInstanceOf(Error)
    })

    it('includes file path in message', () => {
      const err = new ManifestValidationError('/path/to/.skills.toml', 'missing [skills] table')
      expect(err.message).toContain('/path/to/.skills.toml')
    })

    it('includes reason in message', () => {
      const err = new ManifestValidationError('/path/to/.skills.toml', 'missing [skills] table')
      expect(err.message).toContain('missing [skills] table')
    })

    it('has non-empty hint string', () => {
      const err = new ManifestValidationError('/path/to/.skills.toml', 'missing [skills] table')
      expect(err.hint).toBeTruthy()
      expect(typeof err.hint).toBe('string')
    })
  })
})
