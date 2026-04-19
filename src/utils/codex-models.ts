import type { CodexModelOption } from '@/types'

export const FIXED_CODEX_MODEL_VALUES = ['gpt-5.4', 'gpt-5.4-mini', 'gpt-5.3-codex'] as const

export const FIXED_CODEX_MODEL_OPTIONS: CodexModelOption[] = FIXED_CODEX_MODEL_VALUES.map(
  (model) => ({
    label: model,
    value: model,
  }),
)

export function isFixedCodexModel(value?: string | null): value is (typeof FIXED_CODEX_MODEL_VALUES)[number] {
  return Boolean(value && FIXED_CODEX_MODEL_VALUES.includes(value as (typeof FIXED_CODEX_MODEL_VALUES)[number]))
}
