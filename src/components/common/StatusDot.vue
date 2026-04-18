<template>
  <div class="status-dot-wrap" :title="STATUS_LABELS[status]">
    <span class="status-dot" :class="status" />
    <span v-if="showLabel" class="status-label">{{ STATUS_LABELS[status] }}</span>
  </div>
</template>

<script setup lang="ts">
import { STATUS_LABELS } from '@/types'
import type { AccountStatus } from '@/types'

defineProps<{
  status: AccountStatus
  showLabel?: boolean
}>()
</script>

<style scoped>
.status-dot-wrap {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
  box-shadow: 0 0 0 4px transparent;
}

.status-dot.normal {
  background: var(--status-normal);
  box-shadow: 0 0 0 4px var(--status-normal-soft);
}

.status-dot.warning {
  background: var(--status-warning);
  box-shadow: 0 0 0 4px var(--status-warning-soft);
}

.status-dot.error {
  background: var(--status-error);
  box-shadow: 0 0 0 4px var(--status-error-soft);
}

.status-dot.expired {
  background: var(--status-expired);
  box-shadow: 0 0 0 4px var(--status-expired-soft);
}

.status-dot.unknown {
  background: var(--status-unknown);
  box-shadow: 0 0 0 4px var(--status-unknown-soft);
}

.status-label {
  font-size: 12px;
  line-height: 1.33;
  letter-spacing: -0.12px;
  color: var(--app-ink-secondary);
}
</style>
