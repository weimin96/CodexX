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
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.normal  { background: #18a058; box-shadow: 0 0 0 2px rgba(24,160,88,0.25); }
.status-dot.warning { background: #f0a020; box-shadow: 0 0 0 2px rgba(240,160,32,0.25); }
.status-dot.error   { background: #d03050; box-shadow: 0 0 0 2px rgba(208,48,80,0.25); }
.status-dot.expired { background: #8b5cf6; box-shadow: 0 0 0 2px rgba(139,92,246,0.25); }
.status-dot.unknown { background: #909399; }

/* Pulse animation for normal */
.status-dot.normal {
  animation: pulse-green 2.5s infinite;
}
@keyframes pulse-green {
  0%, 100% { box-shadow: 0 0 0 2px rgba(24,160,88,0.25); }
  50% { box-shadow: 0 0 0 5px rgba(24,160,88,0.08); }
}

.status-label {
  font-size: 12px;
  color: var(--text-secondary);
}
</style>
