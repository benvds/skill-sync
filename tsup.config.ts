import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ['bin/skills-sync.ts'],
  format: ['esm'],
  target: 'node20',
  clean: true,
  shims: true,
  banner: {
    js: '#!/usr/bin/env node',
  },
})
