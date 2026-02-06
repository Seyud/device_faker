import js from '@eslint/js'
import pluginVue from 'eslint-plugin-vue'
import configPrettier from 'eslint-config-prettier'
import pluginPrettier from 'eslint-plugin-prettier'
import tseslint from 'typescript-eslint'

export default tseslint.config(
  {
    ignores: [
      '**/dist/**',
      '**/dist-ssr/**',
      '**/coverage/**',
      '**/node_modules/**',
      '**/.vscode/**',
      '**/.idea/**',
      '**/auto-imports.d.ts',
      '**/components.d.ts',
    ],
  },

  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs['flat/recommended'],

  {
    plugins: {
      prettier: pluginPrettier,
    },

    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
      globals: {
        // 浏览器全局变量
        window: 'readonly',
        document: 'readonly',
        navigator: 'readonly',
        console: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        requestAnimationFrame: 'readonly',
        cancelAnimationFrame: 'readonly',
        IntersectionObserver: 'readonly',
        HTMLElement: 'readonly',
        HTMLInputElement: 'readonly',
        Event: 'readonly',
        FileReader: 'readonly',
        File: 'readonly',
        FileList: 'readonly',
        Blob: 'readonly',
        btoa: 'readonly',
        atob: 'readonly',
        localStorage: 'readonly',
        sessionStorage: 'readonly',
      },
    },

    rules: {
      // Prettier
      'prettier/prettier': 'warn',

      // Vue 规则
      'vue/multi-word-component-names': 'off',
      'vue/no-v-html': 'warn',
      'vue/require-default-prop': 'warn',
      'vue/require-explicit-emits': 'warn',
      'vue/v-on-event-hyphenation': 'off',
      'vue/component-name-in-template-casing': ['warn', 'PascalCase'],
      'vue/custom-event-name-casing': 'off',
      'vue/no-useless-v-bind': 'warn',
      'vue/no-unused-refs': 'warn',
      'vue/padding-line-between-blocks': 'off',
      'vue/prefer-separate-static-class': 'off',
      'vue/prefer-true-attribute-shorthand': 'off',
      'vue/component-api-style': 'off',
      'vue/block-lang': 'off',
      'vue/define-macros-order': 'off',
      'vue/html-button-has-type': 'off',
      'vue/no-required-prop-with-default': 'warn',
      'vue/no-template-target-blank': 'warn',
      'vue/no-undef-components': 'off',
      'vue/no-undef-properties': 'off',
      'vue/no-unused-properties': 'off',
      'vue/no-unused-emit-declarations': 'off',

      // TypeScript 规则
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/consistent-type-imports': 'off',
      '@typescript-eslint/no-import-type-side-effects': 'off',
      '@typescript-eslint/no-empty-object-type': 'off',

      // 通用代码质量规则
      'no-debugger': 'warn',
      'no-empty': ['error', { allowEmptyCatch: true }],
      'no-process-exit': 'off',
      'no-useless-escape': 'off',
      'prefer-const': [
        'error',
        {
          destructuring: 'all',
        },
      ],
      'no-var': 'error',
      'object-shorthand': ['error', 'always'],
      'prefer-template': 'warn',
      'prefer-arrow-callback': 'warn',
      'prefer-rest-params': 'warn',
      'prefer-spread': 'warn',
      'no-param-reassign': 'off',
    },
  },

  configPrettier
)
