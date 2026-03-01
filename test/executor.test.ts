/**
 * Tests for executor.ts — subprocess invocation with dry-run boundary.
 * Mocks execa to avoid real subprocess calls.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Mock execa before importing executor
vi.mock('execa', () => ({
  execa: vi.fn(),
  ExecaError: class ExecaError extends Error {
    exitCode?: number
    constructor(message: string) {
      super(message)
      this.name = 'ExecaError'
    }
  },
}))

// Mock reporter to spy on calls without real console output
vi.mock('../src/reporter.js', () => ({
  reporter: {
    printAction: vi.fn(),
    printSuccess: vi.fn(),
    printUnchanged: vi.fn(),
    warnSourceFailed: vi.fn(),
    printCommand: vi.fn(),
    printDryRunCommand: vi.fn(),
    printSummary: vi.fn(),
    printDryRunSummary: vi.fn(),
    printError: vi.fn(),
    warnLockVersionMismatch: vi.fn(),
    warnLockFileCorrupt: vi.fn(),
  },
}))

import { execa } from 'execa'
import { reporter } from '../src/reporter.js'
import { executeSync } from '../src/executor.js'
import type { Diff, SyncOptions } from '../src/types.js'

const mockExeca = vi.mocked(execa)
const mockReporter = vi.mocked(reporter)

function makeDiff(toAdd: string[] = [], unchanged: string[] = []): Diff {
  return { toAdd, unchanged, toRemove: [] }
}

function makeOptions(overrides: Partial<SyncOptions> = {}): SyncOptions {
  return { scope: 'local', dryRun: false, ...overrides }
}

describe('executeSync', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Default: execa resolves successfully
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockExeca.mockResolvedValue(undefined as any)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('dry-run mode', () => {
    it('does NOT call execa in dry-run mode', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions({ dryRun: true }))
      expect(mockExeca).not.toHaveBeenCalled()
    })

    it('calls reporter.printDryRunCommand in dry-run mode', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions({ dryRun: true }))
      expect(mockReporter.printDryRunCommand).toHaveBeenCalledOnce()
    })

    it('does not call reporter.printDryRunCommand when no sources to add', async () => {
      const diff = makeDiff([])
      await executeSync(diff, makeOptions({ dryRun: true }))
      expect(mockReporter.printDryRunCommand).not.toHaveBeenCalled()
    })
  })

  describe('command arguments', () => {
    it('always includes --yes in args', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions())
      expect(mockExeca).toHaveBeenCalledOnce()
      const args = mockExeca.mock.calls[0][1] as string[]
      expect(args).toContain('--yes')
    })

    it('includes --global when scope is global', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions({ scope: 'global' }))
      const args = mockExeca.mock.calls[0][1] as string[]
      expect(args).toContain('--global')
    })

    it('does NOT include --global when scope is local', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions({ scope: 'local' }))
      const args = mockExeca.mock.calls[0][1] as string[]
      expect(args).not.toContain('--global')
    })

    it('includes source in args', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions())
      const args = mockExeca.mock.calls[0][1] as string[]
      expect(args).toContain('owner/repo')
    })

    it('calls npx as the command', async () => {
      const diff = makeDiff(['owner/repo'])
      await executeSync(diff, makeOptions())
      expect(mockExeca.mock.calls[0][0]).toBe('npx')
    })
  })

  describe('failure handling', () => {
    it('continues to next source when one fails', async () => {
      const { ExecaError } = await import('execa')
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      mockExeca
        .mockRejectedValueOnce(new (ExecaError as any)('Command failed'))
        .mockResolvedValueOnce(undefined as any)

      const diff = makeDiff(['failing/source', 'working/source'])
      const result = await executeSync(diff, makeOptions())

      // Both sources attempted
      expect(mockExeca).toHaveBeenCalledTimes(2)
      expect(result.failed).toBe(1)
      expect(result.installed).toBe(1)
    })

    it('calls warnSourceFailed when a source fails', async () => {
      const { ExecaError } = await import('execa')
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      mockExeca.mockRejectedValueOnce(new (ExecaError as any)('exit code 1'))

      const diff = makeDiff(['failing/source'])
      await executeSync(diff, makeOptions())

      expect(mockReporter.warnSourceFailed).toHaveBeenCalledOnce()
      expect(mockReporter.warnSourceFailed).toHaveBeenCalledWith(
        'failing/source',
        expect.any(String),
      )
    })

    it('does not call printSuccess when source fails', async () => {
      const { ExecaError } = await import('execa')
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      mockExeca.mockRejectedValueOnce(new (ExecaError as any)('failed'))

      const diff = makeDiff(['failing/source'])
      await executeSync(diff, makeOptions())

      expect(mockReporter.printSuccess).not.toHaveBeenCalled()
    })
  })

  describe('return value (SyncResult)', () => {
    it('returns installed count equal to successful installs', async () => {
      const diff = makeDiff(['source1', 'source2'])
      const result = await executeSync(diff, makeOptions())
      expect(result.installed).toBe(2)
    })

    it('returns unchanged count from diff.unchanged', async () => {
      const diff = makeDiff([], ['already/here'])
      const result = await executeSync(diff, makeOptions())
      expect(result.unchanged).toBe(1)
    })

    it('returns zero failed on clean run', async () => {
      const diff = makeDiff(['source1'])
      const result = await executeSync(diff, makeOptions())
      expect(result.failed).toBe(0)
    })

    it('dry-run: installed is 0, failed is 0, unchanged from diff', async () => {
      const diff = makeDiff(['source1'], ['already/here'])
      const result = await executeSync(diff, makeOptions({ dryRun: true }))
      expect(result.installed).toBe(0)
      expect(result.failed).toBe(0)
      expect(result.unchanged).toBe(1)
    })
  })

  describe('unchanged sources', () => {
    it('calls printUnchanged for each unchanged source', async () => {
      const diff = makeDiff([], ['already/here', 'also/here'])
      await executeSync(diff, makeOptions())
      expect(mockReporter.printUnchanged).toHaveBeenCalledTimes(2)
    })
  })
})
