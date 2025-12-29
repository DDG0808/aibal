<script setup lang="ts">
/**
 * 全局设置视图
 * Phase 8.2: 插件配置、通知设置、关于
 */
import { ref, computed, watch, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import { AppLayout } from '@/components/layout';
import type { PluginInfo } from '@/types';

const route = useRoute();

// 模拟插件列表
const plugins = ref<PluginInfo[]>([
  {
    id: 'claude-usage',
    name: 'Claude 用量监控',
    version: '1.2.0',
    pluginType: 'data',
    dataType: 'usage',
    enabled: true,
    healthy: true,
    author: 'CUK Official',
    description: '配置连接参数与监控规则',
    icon: 'bolt',
  },
]);

// 当前选中的插件
const selectedPluginId = ref<string>('claude-usage');

// 插件配置
const pluginConfig = ref<Record<string, unknown>>({
  sessionKey: '••••••••••••••••••••••••',
  refreshInterval: 30000,
  backgroundMonitoring: true,
});

// 是否有未保存的更改
const hasChanges = ref(false);

// 当前插件
const selectedPlugin = computed(() => plugins.value.find(p => p.id === selectedPluginId.value));

// 监听配置变化
watch(pluginConfig, () => {
  hasChanges.value = true;
}, { deep: true });

// 保存配置
async function saveConfig() {
  // TODO: 调用 IPC set_plugin_config
  console.log('Saving config:', pluginConfig.value);
  hasChanges.value = false;
}

// 恢复默认
function resetConfig() {
  pluginConfig.value = {
    sessionKey: '',
    refreshInterval: 30000,
    backgroundMonitoring: true,
  };
}

// 从路由参数加载插件
onMounted(() => {
  const pluginId = route.query.plugin as string;
  if (pluginId) {
    selectedPluginId.value = pluginId;
  }
});

// 面包屑导航
const breadcrumbs = computed(() => [
  { label: '我的插件', path: '/plugins' },
  { label: selectedPlugin.value?.name ?? '插件配置', path: '' },
]);
</script>

<template>
  <AppLayout>
    <template #title>
      <h1>全局设置</h1>
    </template>

    <div class="settings-page">
      <!-- 面包屑 -->
      <nav class="breadcrumbs">
        <span v-for="(crumb, index) in breadcrumbs" :key="crumb.label">
          <router-link v-if="crumb.path" :to="crumb.path" class="breadcrumb-link">
            {{ crumb.label }}
          </router-link>
          <span v-else class="breadcrumb-current">{{ crumb.label }}</span>
          <span v-if="index < breadcrumbs.length - 1" class="breadcrumb-separator">›</span>
        </span>
      </nav>

      <!-- 配置卡片 -->
      <div class="config-card">
        <div class="config-header">
          <div class="plugin-icon" style="background: var(--color-accent);">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="config-title">
            <h2>{{ selectedPlugin?.name }}配置</h2>
            <p>配置连接参数与监控规则</p>
          </div>
        </div>

        <div class="config-form">
          <!-- 会话密钥 -->
          <div class="form-field">
            <div class="field-header">
              <label class="field-label">会话密钥 (Session Key)</label>
              <span class="field-required">必填</span>
            </div>
            <div class="field-input-wrapper">
              <input
                v-model="pluginConfig.sessionKey"
                type="password"
                class="field-input"
                placeholder="从 anthropic.com cookie 中获取"
              >
              <span class="field-indicator">已加密</span>
            </div>
            <p class="field-hint">从 anthropic.com cookie 中获取的安全密钥。</p>
          </div>

          <!-- 刷新间隔 -->
          <div class="form-field">
            <label class="field-label">刷新间隔 (毫秒)</label>
            <div class="slider-field">
              <input
                v-model.number="pluginConfig.refreshInterval"
                type="range"
                min="5000"
                max="60000"
                step="1000"
                class="slider"
              >
              <span class="slider-value">{{ pluginConfig.refreshInterval }} ms</span>
            </div>
          </div>

          <!-- 后台监控 -->
          <div class="form-field toggle-field">
            <div class="toggle-info">
              <span class="toggle-label">后台监控</span>
              <span class="toggle-desc">窗口关闭后继续在后台获取数据</span>
            </div>
            <label class="toggle">
              <input
                v-model="pluginConfig.backgroundMonitoring"
                type="checkbox"
              >
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="config-actions">
          <button class="btn btn-secondary" @click="resetConfig">
            恢复默认
          </button>
          <button
            class="btn btn-primary"
            :disabled="!hasChanges"
            @click="saveConfig"
          >
            保存修改
          </button>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
.settings-page {
  max-width: 700px;
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-lg);
  font-size: 0.875rem;
}

.breadcrumb-link {
  color: var(--color-text-secondary);
  text-decoration: none;
  transition: color var(--transition-fast);
}

.breadcrumb-link:hover {
  color: var(--color-text);
}

.breadcrumb-separator {
  color: var(--color-text-tertiary);
}

.breadcrumb-current {
  color: var(--color-text);
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

.plugin-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.config-title h2 {
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

.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.field-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.field-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.field-required {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--color-accent);
  background: rgba(217, 119, 6, 0.15);
  padding: 2px var(--spacing-sm);
  border-radius: var(--radius-sm);
}

.field-input-wrapper {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
  padding: var(--spacing-sm) var(--spacing-md);
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

.field-indicator {
  font-size: 0.75rem;
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
  min-width: 100px;
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
