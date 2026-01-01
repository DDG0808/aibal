<script setup lang="ts">
/**
 * 全局设置视图
 * 包含：插件市场设置、通用设置、检查更新
 */
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
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

// ============================================================================
// 检查更新
// ============================================================================

const GITHUB_REPO = 'DDG0808/aibal';
const currentVersion = ref('0.1.0');
const latestVersion = ref<string | null>(null);
const updateChecking = ref(false);
const updateMessage = ref<{ type: 'success' | 'error' | 'info'; text: string } | null>(null);
const hasUpdate = ref(false);
const releaseUrl = ref<string | null>(null);

// 初始化
onMounted(async () => {
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

  // 获取当前版本
  try {
    currentVersion.value = await invoke<string>('get_version');
  } catch (e) {
    console.error('获取版本失败:', e);
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

// 比较版本号
function compareVersions(a: string, b: string): number {
  const pa = a.replace(/^v/, '').split('.').map(Number);
  const pb = b.replace(/^v/, '').split('.').map(Number);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const na = pa[i] ?? 0;
    const nb = pb[i] ?? 0;
    if (na < nb) return -1;
    if (na > nb) return 1;
  }
  return 0;
}

// 检查更新
async function checkForUpdates() {
  updateChecking.value = true;
  updateMessage.value = null;
  hasUpdate.value = false;
  latestVersion.value = null;
  releaseUrl.value = null;

  try {
    const response = await fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`, {
      headers: {
        'Accept': 'application/vnd.github.v3+json',
      },
    });

    if (!response.ok) {
      if (response.status === 404) {
        updateMessage.value = { type: 'info', text: '暂无发布版本' };
      } else {
        throw new Error(`HTTP ${response.status}`);
      }
      return;
    }

    const data = await response.json();
    const tagName = data.tag_name as string;
    latestVersion.value = tagName.replace(/^v/, '');
    releaseUrl.value = data.html_url as string;

    if (compareVersions(currentVersion.value, latestVersion.value) < 0) {
      hasUpdate.value = true;
      updateMessage.value = {
        type: 'success',
        text: `发现新版本 ${latestVersion.value}`,
      };
    } else {
      updateMessage.value = { type: 'info', text: '已是最新版本' };
    }
  } catch (e) {
    console.error('检查更新失败:', e);
    updateMessage.value = {
      type: 'error',
      text: '检查更新失败，请检查网络连接',
    };
  } finally {
    updateChecking.value = false;
  }
}

// 打开下载页面
function openReleasePage() {
  if (releaseUrl.value) {
    window.open(releaseUrl.value, '_blank');
  }
}
</script>

<template>
  <AppLayout>
    <template #title>
      <h2>设置</h2>
    </template>

    <div class="settings-container">
      <!-- 插件市场 Section -->
      <section class="settings-section">
        <div class="section-header">
          <div class="section-icon blue">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
              <polyline points="9 22 9 12 15 12 15 22" />
            </svg>
          </div>
          <span class="section-title">插件市场</span>
        </div>

        <div class="settings-card">
          <!-- 消息提示 -->
          <Transition name="fade">
            <div v-if="registryMessage" class="toast" :class="registryMessage.type">
              {{ registryMessage.text }}
            </div>
          </Transition>

          <div class="setting-item">
            <div class="setting-row">
              <div class="setting-label">
                <span class="label-main">仓库地址</span>
                <span class="label-sub">插件清单文件 manifest.json 地址</span>
              </div>
              <button class="btn-link" @click="resetRegistryUrl">重置</button>
            </div>
            <div class="input-group">
              <input
                v-model="registryUrl"
                type="url"
                class="input-field"
                placeholder="https://github.com/cuk-team/cuk-plugins"
              >
              <button
                class="btn-save"
                :disabled="registryUrlSaving"
                @click="saveRegistryUrl"
              >
                {{ registryUrlSaving ? '...' : '保存' }}
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- 通用设置 Section -->
      <section class="settings-section">
        <div class="section-header">
          <div class="section-icon gray">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </div>
          <span class="section-title">通用</span>
          <div class="section-actions">
            <button class="btn-link" @click="resetGeneralSettings">重置</button>
            <button
              class="btn-primary-sm"
              :disabled="generalSaving"
              @click="saveGeneralSettings"
            >
              {{ generalSaving ? '保存中...' : '保存' }}
            </button>
          </div>
        </div>

        <div class="settings-card">
          <!-- 消息提示 -->
          <Transition name="fade">
            <div v-if="generalMessage" class="toast" :class="generalMessage.type">
              {{ generalMessage.text }}
            </div>
          </Transition>

          <div class="setting-item with-border">
            <div class="setting-row">
              <div class="setting-label">
                <span class="label-main">刷新间隔</span>
                <span class="label-sub">默认数据刷新频率，单独设置可覆盖</span>
              </div>
              <div class="value-display">{{ globalRefreshInterval }} 分钟</div>
            </div>
            <div class="slider-wrapper">
              <span class="slider-min">1</span>
              <input
                v-model.number="globalRefreshInterval"
                type="range"
                min="1"
                max="60"
                step="1"
                class="slider"
              >
              <span class="slider-max">60</span>
            </div>
          </div>

          <div class="setting-item">
            <div class="setting-row">
              <div class="setting-label">
                <span class="label-main">后台运行</span>
                <span class="label-sub">窗口关闭后继续在后台获取数据</span>
              </div>
              <label class="switch">
                <input v-model="backgroundMonitoring" type="checkbox">
                <span class="switch-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </section>

      <!-- 关于 Section -->
      <section class="settings-section">
        <div class="section-header">
          <div class="section-icon gradient">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="16" x2="12" y2="12" />
              <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
          </div>
          <span class="section-title">关于</span>
        </div>

        <div class="settings-card">
          <!-- 消息提示 -->
          <Transition name="fade">
            <div v-if="updateMessage" class="toast" :class="updateMessage.type">
              {{ updateMessage.text }}
            </div>
          </Transition>

          <div class="setting-item with-border">
            <div class="setting-row">
              <div class="setting-label">
                <span class="label-main">当前版本</span>
              </div>
              <div class="version-badge">v{{ currentVersion }}</div>
            </div>
          </div>

          <div class="setting-item">
            <div class="setting-row">
              <div class="setting-label">
                <span class="label-main">检查更新</span>
                <span v-if="hasUpdate && latestVersion" class="label-sub update-available">
                  新版本 v{{ latestVersion }} 可用
                </span>
                <span v-else class="label-sub">从 GitHub 获取最新版本</span>
              </div>
              <div class="action-buttons">
                <button
                  v-if="hasUpdate"
                  class="btn-primary-sm"
                  @click="openReleasePage"
                >
                  下载
                </button>
                <button
                  class="btn-check"
                  :class="{ loading: updateChecking }"
                  :disabled="updateChecking"
                  @click="checkForUpdates"
                >
                  <svg v-if="!updateChecking" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                    <path d="M3 3v5h5" />
                    <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                    <path d="M16 16h5v5" />
                  </svg>
                  <span v-else class="spinner"></span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  </AppLayout>
</template>

<style scoped>
.settings-container {
  max-width: 800px;
  display: flex;
  flex-direction: column;
  gap: 28px;
}

/* Section */
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 4px;
}

.section-icon {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  flex-shrink: 0;
}

.section-icon.blue {
  background: linear-gradient(135deg, #007AFF, #5856D6);
}

.section-icon.gray {
  background: linear-gradient(135deg, #8E8E93, #636366);
}

.section-icon.gradient {
  background: linear-gradient(135deg, #FF9500, #FF2D55);
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  flex: 1;
}

.section-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Card */
.settings-card {
  background: var(--color-bg-card);
  border-radius: 12px;
  overflow: hidden;
  position: relative;
}

/* Toast */
.toast {
  padding: 10px 14px;
  font-size: 13px;
  font-weight: 500;
  border-bottom: 1px solid var(--color-border);
}

.toast.success {
  background: rgba(52, 199, 89, 0.12);
  color: #34C759;
}

.toast.error {
  background: rgba(255, 59, 48, 0.12);
  color: #FF3B30;
}

.toast.info {
  background: rgba(0, 122, 255, 0.12);
  color: #007AFF;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

/* Setting Item */
.setting-item {
  padding: 14px 16px;
}

.setting-item.with-border {
  border-bottom: 1px solid var(--color-border);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-label {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.label-main {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text);
}

.label-sub {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.label-sub.update-available {
  color: #34C759;
  font-weight: 500;
}

/* Input */
.input-group {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.input-field {
  flex: 1;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 10px 12px;
  font-size: 13px;
  color: var(--color-text);
  transition: border-color 0.15s ease;
}

.input-field:focus {
  outline: none;
  border-color: #007AFF;
}

.input-field::placeholder {
  color: var(--color-text-tertiary);
}

/* Buttons */
.btn-link {
  background: none;
  border: none;
  color: #007AFF;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: background 0.15s ease;
}

.btn-link:hover {
  background: rgba(0, 122, 255, 0.1);
}

.btn-save {
  background: var(--color-bg-tertiary);
  border: none;
  border-radius: 8px;
  padding: 10px 16px;
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text);
  cursor: pointer;
  transition: background 0.15s ease;
  min-width: 56px;
}

.btn-save:hover:not(:disabled) {
  background: var(--color-bg-hover);
}

.btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary-sm {
  background: #007AFF;
  border: none;
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 500;
  color: white;
  cursor: pointer;
  transition: background 0.15s ease;
}

.btn-primary-sm:hover:not(:disabled) {
  background: #0066CC;
}

.btn-primary-sm:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-check {
  width: 32px;
  height: 32px;
  background: var(--color-bg-tertiary);
  border: none;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all 0.15s ease;
}

.btn-check:hover:not(:disabled) {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.btn-check:disabled {
  cursor: not-allowed;
}

.btn-check.loading {
  background: rgba(0, 122, 255, 0.1);
}

.action-buttons {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Value Display */
.value-display {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-secondary);
  background: var(--color-bg-secondary);
  padding: 4px 10px;
  border-radius: 6px;
  min-width: 72px;
  text-align: center;
}

.version-badge {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-secondary);
  background: var(--color-bg-secondary);
  padding: 4px 10px;
  border-radius: 6px;
  font-family: 'SF Mono', Monaco, monospace;
}

/* Slider */
.slider-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.slider-min,
.slider-max {
  font-size: 11px;
  color: var(--color-text-tertiary);
  min-width: 20px;
  text-align: center;
}

.slider {
  flex: 1;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--color-bg-tertiary);
  border-radius: 2px;
  cursor: pointer;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 18px;
  height: 18px;
  background: white;
  border-radius: 50%;
  cursor: pointer;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  transition: transform 0.1s ease;
}

.slider::-webkit-slider-thumb:hover {
  transform: scale(1.1);
}

.slider::-moz-range-thumb {
  width: 18px;
  height: 18px;
  background: white;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

/* Switch */
.switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 26px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.switch-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-tertiary);
  transition: background-color 0.2s ease;
  border-radius: 26px;
}

.switch-slider:before {
  position: absolute;
  content: "";
  height: 22px;
  width: 22px;
  left: 2px;
  bottom: 2px;
  background-color: white;
  transition: transform 0.2s ease;
  border-radius: 50%;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.switch input:checked + .switch-slider {
  background-color: #34C759;
}

.switch input:checked + .switch-slider:before {
  transform: translateX(18px);
}

/* Spinner */
.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid transparent;
  border-top-color: #007AFF;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
