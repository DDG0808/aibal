<script setup lang="ts">
/**
 * 插件配置弹框
 * 根据 configSchema 动态渲染配置表单
 */
import { ref, onMounted, computed } from 'vue';
import { usePluginStore } from '@/stores';
import type { ConfigFieldSchema } from '@/types';

const props = defineProps<{
  pluginId: string;
  pluginName: string;
  configSchema: Record<string, ConfigFieldSchema>;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const pluginStore = usePluginStore();

// 表单数据
const formData = ref<Record<string, unknown>>({});
const isLoading = ref(true);
const isSaving = ref(false);
const message = ref<{ type: 'success' | 'error'; text: string } | null>(null);
const fieldErrors = ref<Record<string, string>>({});

// 配置字段列表
const fields = computed(() => {
  return Object.entries(props.configSchema).map(([key, schema]) => ({
    key,
    ...schema,
  }));
});

// 是否有配置项
const hasFields = computed(() => fields.value.length > 0);

// 加载现有配置
onMounted(async () => {
  isLoading.value = true;
  try {
    const config = await pluginStore.getPluginConfig(props.pluginId);
    if (config) {
      formData.value = { ...config };
    }
    // 设置默认值
    for (const [key, schema] of Object.entries(props.configSchema)) {
      if (formData.value[key] === undefined && schema.default !== undefined) {
        formData.value[key] = schema.default;
      }
    }
  } finally {
    isLoading.value = false;
  }
});

// 验证表单
async function validateForm(): Promise<boolean> {
  fieldErrors.value = {};
  const result = await pluginStore.validatePluginConfig(props.pluginId, formData.value);
  if (!result.valid) {
    if (result.fieldErrors) {
      fieldErrors.value = result.fieldErrors;
    }
    message.value = { type: 'error', text: result.message ?? '配置验证失败' };
    return false;
  }
  return true;
}

// 保存配置
async function saveConfig() {
  message.value = null;

  if (!(await validateForm())) {
    return;
  }

  isSaving.value = true;
  try {
    const success = await pluginStore.savePluginConfig(props.pluginId, formData.value);
    if (success) {
      message.value = { type: 'success', text: '配置已保存' };
      emit('saved');
      // 延迟关闭
      setTimeout(() => {
        emit('close');
      }, 800);
    } else {
      message.value = { type: 'error', text: pluginStore.error ?? '保存失败' };
    }
  } finally {
    isSaving.value = false;
  }
}

// 关闭弹框
function handleClose() {
  if (!isSaving.value) {
    emit('close');
  }
}

// 获取字段错误
function getFieldError(key: string): string | undefined {
  return fieldErrors.value[key];
}
</script>

<template>
  <Teleport to="body">
    <div
      class="config-overlay"
      @click="handleClose"
    >
      <div
        class="config-dialog"
        @click.stop
      >
        <!-- 标题 -->
        <div class="dialog-header">
          <h3 class="dialog-title">
            配置 {{ pluginName }}
          </h3>
          <button
            class="close-btn"
            :disabled="isSaving"
            @click="handleClose"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <line
                x1="18"
                y1="6"
                x2="6"
                y2="18"
              />
              <line
                x1="6"
                y1="6"
                x2="18"
                y2="18"
              />
            </svg>
          </button>
        </div>

        <!-- 中间内容区域（可滚动） -->
        <div class="dialog-body">
          <!-- 加载状态 -->
          <div
            v-if="isLoading"
            class="loading-state"
          >
            <span class="loading-spinner" />
            <span>加载配置中...</span>
          </div>

          <!-- 无配置项 -->
          <div
            v-else-if="!hasFields"
            class="empty-state"
          >
            <p>该插件没有可配置的选项</p>
          </div>

          <!-- 配置表单 -->
          <div
            v-else
            class="config-form"
          >
          <!-- 状态消息 -->
          <div
            v-if="message"
            class="status-message"
            :class="message.type"
          >
            {{ message.text }}
          </div>

          <!-- 字段列表 -->
          <div
            v-for="field in fields"
            :key="field.key"
            class="form-field"
          >
            <label class="field-label">
              {{ field.label || field.key }}
              <span
                v-if="field.required"
                class="required-mark"
              >*</span>
            </label>

            <!-- String 输入框 -->
            <template v-if="field.type === 'string'">
              <div class="field-input-wrapper">
                <input
                  v-model="formData[field.key]"
                  :type="field.secret ? 'password' : 'text'"
                  class="field-input"
                  :class="{ 'has-error': getFieldError(field.key) }"
                  :placeholder="field.description"
                >
              </div>
            </template>

            <!-- Number 输入框 -->
            <template v-else-if="field.type === 'number'">
              <div class="field-input-wrapper">
                <input
                  v-model.number="formData[field.key]"
                  type="number"
                  class="field-input"
                  :class="{ 'has-error': getFieldError(field.key) }"
                  :min="field.min"
                  :max="field.max"
                  :placeholder="field.description"
                >
              </div>
            </template>

            <!-- Boolean Toggle -->
            <template v-else-if="field.type === 'boolean'">
              <div class="toggle-field">
                <label class="toggle">
                  <input
                    v-model="formData[field.key]"
                    type="checkbox"
                  >
                  <span class="toggle-slider" />
                </label>
              </div>
            </template>

            <!-- Select 下拉 -->
            <template v-else-if="field.type === 'select' && field.options">
              <div class="field-input-wrapper">
                <select
                  v-model="formData[field.key]"
                  class="field-select"
                  :class="{ 'has-error': getFieldError(field.key) }"
                >
                  <option
                    value=""
                    disabled
                  >
                    请选择...
                  </option>
                  <option
                    v-for="opt in field.options"
                    :key="opt.value"
                    :value="opt.value"
                  >
                    {{ opt.label }}
                  </option>
                </select>
              </div>
            </template>

            <!-- 字段描述 -->
            <p
              v-if="field.description && field.type !== 'string' && field.type !== 'number'"
              class="field-hint"
            >
              {{ field.description }}
            </p>

            <!-- 字段错误 -->
            <p
              v-if="getFieldError(field.key)"
              class="field-error"
            >
              {{ getFieldError(field.key) }}
            </p>
          </div>
        </div>
        </div>

        <!-- 操作按钮 -->
        <div
          v-if="!isLoading && hasFields"
          class="dialog-actions"
        >
          <button
            class="btn btn-secondary"
            :disabled="isSaving"
            @click="handleClose"
          >
            取消
          </button>
          <button
            class="btn btn-primary"
            :disabled="isSaving"
            @click="saveConfig"
          >
            {{ isSaving ? '保存中...' : '保存' }}
          </button>
        </div>

        <!-- 无配置项时的关闭按钮 -->
        <div
          v-if="!isLoading && !hasFields"
          class="dialog-actions"
        >
          <button
            class="btn btn-primary"
            @click="handleClose"
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.config-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.config-dialog {
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  min-width: 400px;
  max-width: 500px;
  max-height: 80vh;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--spacing-xl);
  padding-bottom: var(--spacing-lg);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.dialog-title {
  margin: 0;
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.close-btn:hover:not(:disabled) {
  background: var(--color-bg-tertiary);
  color: var(--color-text);
}

.close-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.dialog-body {
  flex: 1;
  min-height: 0; /* 关键：确保 flexbox 子元素可以滚动 */
  overflow-y: auto;
  padding: var(--spacing-xl);
  padding-top: var(--spacing-lg);
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl) 0;
  color: var(--color-text-secondary);
  gap: var(--spacing-md);
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.config-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.status-message {
  padding: var(--spacing-md);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
}

.status-message.success {
  background: rgba(34, 197, 94, 0.1);
  border: 1px solid rgba(34, 197, 94, 0.3);
  color: var(--color-accent-green, #22c55e);
}

.status-message.error {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: var(--color-accent-red, #ef4444);
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.field-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.required-mark {
  color: var(--color-accent-red, #ef4444);
  margin-left: 2px;
}

.field-input-wrapper {
  display: flex;
  align-items: center;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  transition: border-color var(--transition-fast);
}

.field-input-wrapper:focus-within {
  border-color: var(--color-accent);
}

.field-input,
.field-select {
  flex: 1;
  width: 100%;
  background: none;
  border: none;
  font-size: 0.9375rem;
  color: var(--color-text);
  padding: var(--spacing-sm) var(--spacing-md);
}

.field-input:focus,
.field-select:focus {
  outline: none;
}

.field-input::placeholder {
  color: var(--color-text-tertiary);
}

.field-input.has-error,
.field-select.has-error {
  border-color: var(--color-accent-red, #ef4444);
}

.field-select {
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%23666' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 36px;
}

.toggle-field {
  display: flex;
  align-items: center;
}

/* Toggle Switch */
.toggle {
  position: relative;
  display: inline-block;
  width: 48px;
  height: 28px;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-tertiary);
  transition: var(--transition-fast);
  border-radius: 28px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 22px;
  width: 22px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: var(--transition-fast);
  border-radius: 50%;
}

.toggle input:checked + .toggle-slider {
  background-color: var(--color-accent);
}

.toggle input:checked + .toggle-slider:before {
  transform: translateX(20px);
}

.field-hint {
  font-size: 0.8125rem;
  color: var(--color-text-tertiary);
  margin: 0;
}

.field-error {
  font-size: 0.8125rem;
  color: var(--color-accent-red, #ef4444);
  margin: 0;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
  padding: var(--spacing-xl);
  padding-top: var(--spacing-lg);
  border-top: 1px solid var(--color-border);
  flex-shrink: 0;
}

.btn {
  padding: var(--spacing-sm) var(--spacing-lg);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-secondary {
  background: none;
  border: none;
  color: var(--color-text-secondary);
}

.btn-secondary:hover:not(:disabled) {
  color: var(--color-text);
}

.btn-primary {
  background: var(--color-text);
  border: none;
  color: var(--color-bg);
}

.btn-primary:hover:not(:disabled) {
  opacity: 0.9;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
