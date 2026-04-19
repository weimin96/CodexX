<template>
  <n-modal
    :show="show"
    preset="card"
    title="新增账号"
    style="width: 560px;"
    :mask-closable="false"
    @update:show="$emit('update:show', $event)"
  >
    <n-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-placement="top"
      label-width="auto"
    >
      <n-form-item label="API Key" path="credential_value">
        <n-input
          v-model:value="form.credential_value"
          type="password"
          placeholder="sk-..."
          show-password-on="click"
        />
      </n-form-item>

      <n-form-item label="邮箱（可选）" path="email">
        <n-input v-model:value="form.email" placeholder="user@example.com" />
      </n-form-item>

      <n-form-item label="标识颜色">
        <div class="color-row">
          <button
            v-for="color in PRESET_COLORS"
            :key="color"
            type="button"
            class="color-dot"
            :class="{ selected: form.color === color }"
            :style="{ background: color }"
            @click="form.color = color"
          />
        </div>
      </n-form-item>

      <n-alert type="info" :show-icon="true" style="margin-bottom: 8px;">
        <template #header>安全存储</template>
        凭证会通过 AES-256-GCM 加密后保存在本地数据库中，不会以明文落盘。
      </n-alert>

      <div class="modal-footer">
        <n-button secondary @click="$emit('update:show', false)">取消</n-button>
        <n-button type="primary" :loading="loading" @click="handleSubmit">创建账号</n-button>
      </div>
    </n-form>
  </n-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { FormInst, FormRules } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import type { Account, AuthType } from '@/types'

defineProps<{ show: boolean }>()
const emit = defineEmits<{
  'update:show': [boolean]
  created: [Account]
}>()

const accountStore = useAccountStore()
const formRef = ref<FormInst | null>(null)
const loading = ref(false)

const PRESET_COLORS = [
  '#0071e3',
  '#1f8f5f',
  '#b26a00',
  '#c4314b',
  '#7254d1',
  '#0f9fb0',
  '#d96a20',
  '#b53b70',
  '#147d68',
  '#64748b',
]

const form = ref({
  auth_type: 'api_key' as AuthType,
  email: '',
  color: '#0071e3',
  credential_value: '',
})

const rules: FormRules = {
  credential_value: [{ required: true, message: '请输入凭证信息', trigger: 'blur' }],
}

async function handleSubmit() {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }

  loading.value = true
  try {
    const account = await accountStore.createAccount({
      auth_type: form.value.auth_type,
      email: form.value.email || undefined,
      color: form.value.color,
      credential_value: form.value.credential_value,
    })
    emit('created', account)
    form.value = {
      auth_type: 'api_key',
      email: '',
      color: '#0071e3',
      credential_value: '',
    }
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.color-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.color-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    box-shadow 0.18s ease;
}

.color-dot:hover {
  transform: scale(1.08);
}

.color-dot.selected {
  border-color: #ffffff;
  box-shadow: 0 0 0 2px rgba(29, 29, 31, 0.16);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 18px;
}
</style>
