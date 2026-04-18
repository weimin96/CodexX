<template>
  <n-modal
    :show="show"
    preset="card"
    title="新增账号"
    style="width: 520px;"
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
      <!-- Name -->
      <n-form-item label="账号名称" path="name">
        <n-input v-model:value="form.name" placeholder="例如：工作账号、个人账号..." />
      </n-form-item>

      <!-- Auth type -->
      <n-form-item label="认证方式" path="auth_type">
        <n-select
          v-model:value="form.auth_type"
          :options="authTypeOptions"
          @update:value="form.credential_value = ''"
        />
      </n-form-item>

      <!-- Credential input -->
      <n-form-item :label="credentialLabel" path="credential_value">
        <n-input
          v-model:value="form.credential_value"
          :type="form.auth_type === 'cookie_session' ? 'textarea' : 'password'"
          :rows="form.auth_type === 'cookie_session' ? 4 : 1"
          :placeholder="credentialPlaceholder"
          show-password-on="click"
        />
      </n-form-item>

      <n-grid :cols="2" :x-gap="12">
        <n-gi>
          <n-form-item label="邮箱（可选）" path="email">
            <n-input v-model:value="form.email" placeholder="user@example.com" />
          </n-form-item>
        </n-gi>
        <n-gi>
          <n-form-item label="组织（可选）" path="organization">
            <n-input v-model:value="form.organization" placeholder="公司 / 团队名称" />
          </n-form-item>
        </n-gi>
      </n-grid>

      <!-- Color picker -->
      <n-form-item label="标识颜色">
        <div class="color-row">
          <div
            v-for="c in PRESET_COLORS"
            :key="c"
            class="color-dot"
            :class="{ selected: form.color === c }"
            :style="{ background: c }"
            @click="form.color = c"
          />
        </div>
      </n-form-item>

      <!-- Security notice -->
      <n-alert type="info" :show-icon="true" style="margin-bottom: 8px;">
        <template #header>安全存储</template>
        密钥将通过 AES-256-GCM 加密后存储在本地，不会明文保存。
      </n-alert>

      <div class="modal-footer">
        <n-button @click="$emit('update:show', false)">取消</n-button>
        <n-button type="primary" :loading="loading" @click="handleSubmit">
          创建账号
        </n-button>
      </div>
    </n-form>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
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
  '#4f8ef7', '#18a058', '#f0a020', '#d03050', '#8b5cf6',
  '#06b6d4', '#f97316', '#ec4899', '#10b981', '#64748b',
]

const form = ref({
  name: '',
  auth_type: 'api_key' as AuthType,
  email: '',
  organization: '',
  color: '#4f8ef7',
  credential_value: '',
})

const authTypeOptions = [
  { label: 'API Key', value: 'api_key' },
  { label: 'OAuth / Token', value: 'oauth_token' },
  { label: 'Cookie / Session', value: 'cookie_session' },
  { label: 'CLI Profile', value: 'cli_profile' },
]

const credentialLabel = computed(() => {
  const labels: Record<AuthType, string> = {
    api_key: 'API Key',
    oauth_token: 'OAuth Token',
    cookie_session: 'Cookie / Session',
    cli_profile: 'CLI Profile 名称',
  }
  return labels[form.value.auth_type]
})

const credentialPlaceholder = computed(() => {
  const placeholders: Record<AuthType, string> = {
    api_key: 'sk-...',
    oauth_token: 'Bearer token...',
    cookie_session: '粘贴 Cookie 内容...',
    cli_profile: 'default',
  }
  return placeholders[form.value.auth_type]
})

const rules: FormRules = {
  name: [{ required: true, message: '请输入账号名称', trigger: 'blur' }],
  auth_type: [{ required: true, message: '请选择认证方式' }],
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
      name: form.value.name,
      auth_type: form.value.auth_type,
      email: form.value.email || undefined,
      organization: form.value.organization || undefined,
      color: form.value.color,
      credential_value: form.value.credential_value,
    })
    emit('created', account)
    // Reset form
    form.value = {
      name: '',
      auth_type: 'api_key',
      email: '',
      organization: '',
      color: '#4f8ef7',
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
  gap: 8px;
  flex-wrap: wrap;
}

.color-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid transparent;
  transition: all 0.15s;
}
.color-dot:hover { transform: scale(1.15); }
.color-dot.selected {
  border-color: #fff;
  box-shadow: 0 0 0 2px rgba(255,255,255,0.3);
  transform: scale(1.15);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
</style>
