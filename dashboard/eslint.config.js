import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      // Widgets deliberately co-locate their settings constants/types with
      // the component (Tire/Electronics/… + *Settings), and registry.tsx
      // wraps all widgets in memo() next to the REGISTRY table. The rule
      // only affects Vite HMR granularity (full reload instead of fast
      // refresh for those modules), not correctness.
      'react-refresh/only-export-components': 'off',
    },
  },
])
