<script setup lang="ts">
/**
 * 全局设置视图
 * 包含：插件市场设置、通用设置
 */
import { ref, onMounted } from 'vue';
import { AppLayout } from '@/components/layout';
import { marketplaceService } from '@/services/marketplace';

// ============================================================================
// 插件市场设置
// ============================================================================

const registryUrl = ref('');
const registryUrlSaving = ref(false);
const registryMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null);

// ============================================================================
// 通用设置
// ============================================================================

const globalRefreshInterval = ref(30); // 分钟
const backgroundMonitoring = ref(true);
const generalSaving = ref(false);
const generalMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null);

// 初始化
onMounted(() => {
  // 加载市场 URL
  registryUrl.value = marketplaceService.getRegistryUrl();

  // 加载通用设置
  const savedInterval = localStorage.getItem('globalRefreshInterval');
  if (savedInterval) {
    globalRefreshInterval.value = parseInt(savedInterval, 10);
  }
  const savedBgMonitor = localStorage.getItem('backgroundMonitoring');
  if (savedBgMonitor !== null) {
    backgroundMonitoring.value = savedBgMonitor === 'true';
  }
});

// 保存市场设置
async function saveRegistryUrl() {
  registryUrlSaving.value = true;
  registryMessage.value = null;

  try {
    // 验证 URL 格式（如果不为空）
    if (registryUrl.value.trim()) {
      try {
        new URL(registryUrl.value.trim());
      } catch {
        registryMessage.value = { type: 'error', text: 'URL 格式无效' };
        return;
      }
    }

    marketplaceService.setRegistryUrl(registryUrl.value.trim() || null);
    registryMessage.value = { type: 'success', text: '已保存，刷新市场页面生效' };

    setTimeout(() => {
      registryMessage.value = null;
    }, 3000);
  } finally {
    registryUrlSaving.value = false;
  }
}

// 恢复市场默认设置
function resetRegistryUrl() {
  marketplaceService.setRegistryUrl(null);
  registryUrl.value = marketplaceService.getRegistryUrl();
  registryMessage.value = { type: 'success', text: '已恢复默认地址' };
  setTimeout(() => {
    registryMessage.value = null;
  }, 3000);
}

// 保存通用设置
async function saveGeneralSettings() {
  generalSaving.value = true;
  generalMessage.value = null;

  try {
    localStorage.setItem('globalRefreshInterval', String(globalRefreshInterval.value));
    localStorage.setItem('backgroundMonitoring', String(backgroundMonitoring.value));
    generalMessage.value = { type: 'success', text: '设置已保存' };

    setTimeout(() => {
      generalMessage.value = null;
    }, 3000);
  } finally {
    generalSaving.value = false;
  }
}

// 恢复通用默认设置
function resetGeneralSettings() {
  globalRefreshInterval.value = 30;
  backgroundMonitoring.value = true;
  localStorage.removeItem('globalRefreshInterval');
  localStorage.removeItem('backgroundMonitoring');
  generalMessage.value = { type: 'success', text: '已恢复默认设置' };
  setTimeout(() => {
    generalMessage.value = null;
  }, 3000);
}
</script>

<template>
  <AppLayout>
    <template #title>
      <h2>全局设置</h2>
    </template>

    <div class="settings-page">
      <!-- 插件市场设置 -->
      <div class="config-card">
        <div class="config-header">
          <div class="config-icon marketplace-icon">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
              <polyline points="9 22 9 12 15 12 15 22" />
            </svg>
          </div>
          <div class="config-title">
            <h3>插件市场设置</h3>
            <p>配置插件来源与更新策略</p>
          </div>
        </div>

        <div class="config-form">
          <!-- 状态消息 -->
          <div
            v-if="registryMessage"
            class="status-message"
            :class="registryMessage.type"
          >
            {{ registryMessage.text }}
          </div>

          <!-- Registry URL -->
          <div class="form-field">
            <label class="field-label">插件仓库地址 (Registry URL)</label>
            <div class="field-input-wrapper with-icon">
              <svg
                class="input-icon"
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="10"
                />
                <line
                  x1="2"
                  y1="12"
                  x2="22"
                  y2="12"
                />
                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
              </svg>
              <input
                v-model="registryUrl"
                type="url"
                class="field-input"
                placeholder="https://github.com/cuk-team/cuk-plugins"
              >
            </div>
            <p class="field-hint">
              官方或第三方托管的插件清单文件地址 (manifest.json)。
            </p>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="config-actions">
          <button
            class="btn btn-secondary"
            :disabled="registryUrlSaving"
            @click="resetRegistryUrl"
          >
            恢复默认
          </button>
          <button
            class="btn btn-primary"
            :disabled="registryUrlSaving"
            @click="saveRegistryUrl"
          >
            {{ registryUrlSaving ? '保存中...' : '保存设置' }}
          </button>
        </div>
      </div>

      <!-- 通用设置 -->
      <div class="config-card">
        <div class="config-header">
          <div class="config-icon general-icon">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle
                cx="12"
                cy="12"
                r="3"
              />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </div>
          <div class="config-title">
            <h3>通用设置</h3>
            <p>应用行为与后台任务管理</p>
          </div>
        </div>

        <div class="config-form">
          <!-- 状态消息 -->
          <div
            v-if="generalMessage"
            class="status-message"
            :class="generalMessage.type"
          >
            {{ generalMessage.text }}
          </div>

          <!-- 全局刷新间隔 -->
          <div class="form-field">
            <label class="field-label">全局刷新间隔 (默认)</label>
            <div class="slider-field">
              <input
                v-model.number="globalRefreshInterval"
                type="range"
                min="1"
                max="60"
                step="1"
                class="slider"
              >
              <span class="slider-value">{{ globalRefreshInterval }} 分钟</span>
            </div>
            <p class="field-hint">
              此设置将作为所有插件的默认刷新频率，插件单独设置可覆盖此值。
            </p>
          </div>

          <!-- 后台监控 -->
          <div class="form-field toggle-field">
            <div class="toggle-info">
              <span class="toggle-label">后台监控</span>
              <span class="toggle-desc">关闭窗口后继续在后台运行并获取数据</span>
            </div>
            <label class="toggle">
              <input
                v-model="backgroundMonitoring"
                type="checkbox"
              >
              <span class="toggle-slider" />
            </label>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="config-actions">
          <button
            class="btn btn-secondary"
            :disabled="generalSaving"
            @click="resetGeneralSettings"
          >
            恢复默认
          </button>
          <button
            class="btn btn-primary"
            :disabled="generalSaving"
            @click="saveGeneralSettings"
          >
            {{ generalSaving ? '保存中...' : '保存设置' }}
          </button>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
.settings-page {
  max-width: 700px;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xl);
}

.config-card {
  background: var(--color-bg-card);
  border-radius: var(--radius-xl);
  padding: var(--spacing-xl);
}

.config-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  padding-bottom: var(--spacing-xl);
  border-bottom: 1px solid var(--color-border);
  margin-bottom: var(--spacing-xl);
}

.config-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.marketplace-icon {
  background: var(--color-accent-blue, #3b82f6);
}

.general-icon {
  background: var(--color-bg-tertiary, #374151);
}

.config-title h3 {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0 0 var(--spacing-xs);
}

.config-title p {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin: 0;
}

.config-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xl);
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

.field-input-wrapper {
  display: flex;
  align-items: center;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
  padding: var(--spacing-sm) var(--spacing-md);
}

.field-input-wrapper.with-icon {
  gap: var(--spacing-sm);
}

.input-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.field-input {
  flex: 1;
  background: none;
  border: none;
  font-size: 0.9375rem;
  color: var(--color-text);
  padding: var(--spacing-sm) 0;
}

.field-input:focus {
  outline: none;
}

.field-input::placeholder {
  color: var(--color-text-tertiary);
}

.field-hint {
  font-size: 0.8125rem;
  color: var(--color-text-tertiary);
  margin: 0;
}

.slider-field {
  display: flex;
  align-items: center;
  gap: var(--spacing-lg);
}

.slider {
  flex: 1;
  height: 6px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--color-bg-tertiary);
  border-radius: 3px;
  cursor: pointer;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  background: var(--color-accent);
  border-radius: 50%;
  cursor: pointer;
}

.slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  background: var(--color-accent);
  border-radius: 50%;
  border: none;
  cursor: pointer;
}

.slider-value {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
  background: var(--color-bg-tertiary);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-md);
  min-width: 80px;
  text-align: center;
}

.toggle-field {
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  padding: var(--spacing-md);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
}

.toggle-info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.toggle-label {
  font-size: 0.9375rem;
  font-weight: 500;
  color: var(--color-text);
}

.toggle-desc {
  font-size: 0.8125rem;
  color: var(--color-text-secondary);
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

.config-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
  margin-top: var(--spacing-xl);
  padding-top: var(--spacing-xl);
  border-top: 1px solid var(--color-border);
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

.btn-secondary:hover {
  color: var(--color-text);
}

.btn-primary {
  background: var(--color-text);
  border: none;
  color: var(--color-bg);
}

.btn-primary:hover {
  opacity: 0.9;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
