/**
 * Tests for reporter.ts — terminal output with status symbols and colors.
 * Uses spies on console.log and console.error to verify output format.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { reporter } from '../src/reporter.js'

describe('reporter', () => {
  let logSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    logSpy = vi.spyOn(console, 'log').mockImplementation(() => {})
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('printSummary', () => {
    it('outputs "X installed, Y removed, Z unchanged"', () => {
      reporter.printSummary(2, 0, 3)
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('2 installed')
      expect(output).toContain('0 removed')
      expect(output).toContain('3 unchanged')
    })

    it('works with zero counts', () => {
      reporter.printSummary(0, 0, 0)
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('0 installed')
      expect(output).toContain('0 removed')
      expect(output).toContain('0 unchanged')
    })
  })

  describe('printDryRunSummary', () => {
    it('outputs "X to install, Y to remove, Z unchanged" with (dry run) suffix', () => {
      reporter.printDryRunSummary(2, 0, 3)
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('2 to install')
      expect(output).toContain('0 to remove')
      expect(output).toContain('3 unchanged')
      expect(output).toContain('dry run')
    })

    it('works with zero counts', () => {
      reporter.printDryRunSummary(0, 0, 5)
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('0 to install')
      expect(output).toContain('5 unchanged')
    })
  })

  describe('printAction', () => {
    it('outputs symbol and source for installing', () => {
      reporter.printAction('installing', 'owner/repo')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('owner/repo')
    })

    it('outputs symbol and source for skipped', () => {
      reporter.printAction('skipped', 'owner/repo')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('owner/repo')
    })
  })

  describe('printSuccess', () => {
    it('outputs source name', () => {
      reporter.printSuccess('owner/repo')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('owner/repo')
    })
  })

  describe('printUnchanged', () => {
    it('outputs source name', () => {
      reporter.printUnchanged('owner/repo')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('owner/repo')
    })
  })

  describe('warnSourceFailed', () => {
    it('outputs source and reason to stderr', () => {
      reporter.warnSourceFailed('owner/repo', 'exit code 1')
      expect(errorSpy).toHaveBeenCalledOnce()
      const output = errorSpy.mock.calls[0][0] as string
      expect(output).toContain('owner/repo')
      expect(output).toContain('exit code 1')
    })
  })

  describe('printError', () => {
    it('outputs "Error:" with message and "Hint:" below', () => {
      reporter.printError('something went wrong', 'try this fix')
      expect(errorSpy).toHaveBeenCalledTimes(2)
      const firstLine = errorSpy.mock.calls[0][0] as string
      const secondLine = errorSpy.mock.calls[1][0] as string
      expect(firstLine).toContain('something went wrong')
      expect(secondLine).toContain('try this fix')
    })
  })

  describe('printCommand', () => {
    it('outputs the command string', () => {
      reporter.printCommand('npx skills add owner/repo --yes')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('npx skills add owner/repo --yes')
    })
  })

  describe('printDryRunCommand', () => {
    it('outputs command prefixed with dry run indication', () => {
      reporter.printDryRunCommand('npx skills add owner/repo --yes')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('npx skills add owner/repo --yes')
      expect(output).toContain('dry run')
    })
  })

  describe('warnLockVersionMismatch', () => {
    it('outputs a warning with actual and expected versions', () => {
      reporter.warnLockVersionMismatch(99, 3)
      expect(errorSpy).toHaveBeenCalledOnce()
      const output = errorSpy.mock.calls[0][0] as string
      expect(output).toContain('99')
      expect(output).toContain('3')
    })
  })

  describe('warnLockFileCorrupt', () => {
    it('outputs a warning with the path', () => {
      reporter.warnLockFileCorrupt('/path/to/lock.json')
      expect(errorSpy).toHaveBeenCalledOnce()
      const output = errorSpy.mock.calls[0][0] as string
      expect(output).toContain('/path/to/lock.json')
    })
  })

  describe('printOrphans', () => {
    it('outputs each skill name', () => {
      reporter.printOrphans(['skill-a', 'skill-b'])
      // Header + 2 skill lines = at least 3 calls
      expect(logSpy.mock.calls.length).toBeGreaterThanOrEqual(3)
      const allOutput = logSpy.mock.calls.map((c: unknown[]) => c[0]).join('\n')
      expect(allOutput).toContain('skill-a')
      expect(allOutput).toContain('skill-b')
    })
  })

  describe('printRemoving', () => {
    it('outputs skill name with removing indicator', () => {
      reporter.printRemoving('git-helpers')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('git-helpers')
      expect(output).toContain('removing')
    })
  })

  describe('printRemoved', () => {
    it('outputs skill name', () => {
      reporter.printRemoved('git-helpers')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('git-helpers')
      expect(output).toContain('removed')
    })
  })

  describe('warnRemoveFailed', () => {
    it('outputs to stderr with skill name and reason', () => {
      reporter.warnRemoveFailed('git-helpers', 'exit code 1')
      expect(errorSpy).toHaveBeenCalledOnce()
      const output = errorSpy.mock.calls[0][0] as string
      expect(output).toContain('git-helpers')
      expect(output).toContain('exit code 1')
    })
  })

  describe('warnSkippingOrphans', () => {
    it('outputs reason', () => {
      reporter.warnSkippingOrphans('stdin is not a TTY')
      expect(logSpy).toHaveBeenCalledOnce()
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('stdin is not a TTY')
    })
  })

  describe('printPreSyncPreview', () => {
    it('outputs to-add and to-remove sections', () => {
      reporter.printPreSyncPreview({
        toAdd: ['owner/repo'],
        toRemove: ['orphan-skill'],
        unchanged: ['existing/source'],
      })
      const allOutput = logSpy.mock.calls.map((c: unknown[]) => c[0]).join('\n')
      expect(allOutput).toContain('owner/repo')
      expect(allOutput).toContain('orphan-skill')
    })

    it('skips sections when empty', () => {
      reporter.printPreSyncPreview({ toAdd: [], toRemove: [], unchanged: ['a/b'] })
      expect(logSpy).not.toHaveBeenCalled()
    })
  })

  describe('printSummary with removed count', () => {
    it('includes removed count in output', () => {
      reporter.printSummary(2, 1, 3)
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('2 installed')
      expect(output).toContain('1 removed')
      expect(output).toContain('3 unchanged')
    })
  })

  describe('printDryRunSummary with toRemove count', () => {
    it('includes toRemove count in output', () => {
      reporter.printDryRunSummary(2, 1, 3)
      const output = logSpy.mock.calls[0][0] as string
      expect(output).toContain('2 to install')
      expect(output).toContain('1 to remove')
      expect(output).toContain('3 unchanged')
    })
  })
})
