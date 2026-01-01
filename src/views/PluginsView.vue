<script setup lang="ts">
/**
 * 我的插件视图
 * Phase 8.2: 插件管理、启用/禁用/删除插件
 */
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { AppLayout } from '@/components/layout';
import PluginConfigDialog from '@/components/dialog/PluginConfigDialog.vue';
import { usePluginStore } from '@/stores';
import type { HealthStatus, ConfigFieldSchema } from '@/types';

const pluginStore = usePluginStore();
const router = useRouter();

// 确认对话框状态
const showConfirmDialog = ref(false);
const confirmAction = ref<'uninstall' | 'reload' | null>(null);
const confirmPluginId = ref<string | null>(null);
const confirmPluginName = ref<string>('');
const isProcessing = ref(false);
const operationError = ref<string | null>(null);
const operationSuccess = ref<string | null>(null);

// 操作菜单状态
const activeMenuId = ref<string | null>(null);

// 配置对话框状态
const showConfigDialog = ref(false);
const configPluginId = ref<string | null>(null);
const configPluginName = ref('');
const configSchema = ref<Record<string, ConfigFieldSchema>>({});

// 事件监听器清理函数
let unlistenPluginDisabled: (() => void) | null = null;

// 从 Store 获取插件列表，并计算健康状态
const plugins = computed(() => {
  return pluginStore.plugins.map(plugin => {
    const health = pluginStore.pluginHealth.get(plugin.id);
    return {
      ...plugin,
      calls: health ? Math.floor(health.successRate * 100) : 0,
      successRate: health ? Math.round(health.successRate * 100) : 100,
      latency: health?.avgLatencyMs ? Math.round(health.avgLatencyMs) : 0,
      status: (health?.status ?? 'healthy') as HealthStatus,
    };
  });
});

// 统计数据
const stats = computed(() => ({
  totalPlugins: plugins.value.length,
  enabledPlugins: pluginStore.plugins.filter(p => p.enabled).length,
  systemHealth: pluginStore.systemHealthRate,
  totalCalls: pluginStore.totalCalls,
}));

// 切换插件启用状态
async function togglePlugin(id: string) {
  const plugin = pluginStore.plugins.find(p => p.id === id);
  if (plugin) {
    if (plugin.enabled) {
      await pluginStore.disablePlugin(id);
    } else {
      await pluginStore.enablePlugin(id);
    }
  }
}

// 显示操作菜单
function toggleMenu(id: string) {
  activeMenuId.value = activeMenuId.value === id ? null : id;
}

// 关闭菜单
function closeMenu() {
  activeMenuId.value = null;
}

// 打开确认对话框
function openConfirmDialog(action: 'uninstall' | 'reload', id: string, name: string) {
  confirmAction.value = action;
  confirmPluginId.value = id;
  confirmPluginName.value = name;
  operationError.value = null; // 打开时清理上次错误
  showConfirmDialog.value = true;
  closeMenu();
}

// 关闭确认对话框
function closeConfirmDialog() {
  showConfirmDialog.value = false;
  confirmAction.value = null;
  confirmPluginId.value = null;
  confirmPluginName.value = '';
  operationError.value = null; // 关闭时也清理错误
}

// 执行确认操作
async function executeConfirmAction() {
  if (!confirmPluginId.value || !confirmAction.value) return;

  isProcessing.value = true;
  operationError.value = null;
  operationSuccess.value = null;

  try {
    let success = false;
    const actionName = confirmAction.value === 'uninstall' ? '卸载' : '重载';

    if (confirmAction.value === 'uninstall') {
      success = await pluginStore.uninstallPlugin(confirmPluginId.value);
    } else if (confirmAction.value === 'reload') {
      success = await pluginStore.reloadPlugin(confirmPluginId.value);
    }

    if (success) {
      operationSuccess.value = `${actionName}成功`;
      closeConfirmDialog();
      // 3秒后清除成功提示
      setTimeout(() => { operationSuccess.value = null; }, 3000);
    } else {
      // 失败时保持对话框打开，显示错误
      operationError.value = pluginStore.error ?? `${actionName}失败`;
    }
  } catch (e) {
    operationError.value = e instanceof Error ? e.message : '操作失败';
  } finally {
    isProcessing.value = false;
  }
}

// 跳转到应用市场
function goToMarketplace() {
  router.push('/marketplace');
}

// 打开配置弹框
function configurePlugin(id: string) {
  const plugin = pluginStore.plugins.find(p => p.id === id);
  if (plugin) {
    configPluginId.value = id;
    configPluginName.value = plugin.name;
    configSchema.value = plugin.configSchema ?? {};
    showConfigDialog.value = true;
  }
  closeMenu();
}

// 关闭配置弹框
function closeConfigDialog() {
  showConfigDialog.value = false;
  configPluginId.value = null;
  configPluginName.value = '';
  configSchema.value = {};
}

// 点击空白处关闭菜单
function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (!target.closest('.plugin-menu-wrapper')) {
    closeMenu();
  }
}

// 初始化加载数据
onMounted(async () => {
  // 先添加全局点击监听器（避免 await 后组件已卸载导致泄漏）
  document.addEventListener('click', handleClickOutside);

  // 监听插件禁用事件（如果正在配置的插件被禁用，关闭配置弹框）
  unlistenPluginDisabled = await pluginStore.setupPluginDisabledListener((disabledPluginId) => {
    console.log('[PluginsView] 收到插件禁用事件:', disabledPluginId);
    // 如果正在配置被禁用的插件，关闭配置弹框
    if (showConfigDialog.value && configPluginId.value === disabledPluginId) {
      closeConfigDialog();
    }
    // 关闭操作菜单（如果正在显示被禁用插件的菜单）
    if (activeMenuId.value === disabledPluginId) {
      closeMenu();
    }
  });

  if (pluginStore.plugins.length === 0) {
    await pluginStore.init();
  }
});

// 清理
onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  // 清理插件禁用事件监听器
  if (unlistenPluginDisabled) {
    unlistenPluginDisabled();
    unlistenPluginDisabled = null;
  }
});

// 获取状态标签
function getStatusLabel(status: HealthStatus): string {
  switch (status) {
    case 'healthy': return '运行正常';
    case 'degraded': return '性能降级';
    case 'unhealthy': return '运行异常';
    default: return '未知';
  }
}

// 获取状态颜色类
function getStatusClass(status: HealthStatus): string {
  switch (status) {
    case 'healthy': return 'status-healthy';
    case 'degraded': return 'status-degraded';
    case 'unhealthy': return 'status-unhealthy';
    default: return '';
  }
}

// 获取确认对话框标题
function getConfirmTitle(): string {
  if (confirmAction.value === 'uninstall') {
    return '确认卸载插件';
  } else if (confirmAction.value === 'reload') {
    return '确认重载插件';
  }
  return '';
}

// 获取确认对话框描述
function getConfirmDescription(): string {
  if (confirmAction.value === 'uninstall') {
    return `确定要卸载插件 "${confirmPluginName.value}" 吗？此操作不可撤销。`;
  } else if (confirmAction.value === 'reload') {
    return `确定要重载插件 "${confirmPluginName.value}" 吗？这将重新读取插件配置。`;
  }
  return '';
}
</script>

<template>
  <AppLayout>
    <template #title>
      <h2>我的插件</h2>
    </template>

    <div class="plugins-page">
      <!-- 统计概览 -->
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-header">
            <svg
              class="stat-icon"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M12 2L2 7l10 5 10-5-10-5z"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M2 17l10 5 10-5"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M2 12l10 5 10-5"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <span class="stat-label">插件总数</span>
          </div>
          <div class="stat-value">
            <span class="stat-number">{{ stats.totalPlugins }}</span>
            <span class="stat-change positive">{{ stats.enabledPlugins }} 已启用</span>
          </div>
        </div>

        <div class="stat-card card-green">
          <div class="stat-header">
            <svg
              class="stat-icon"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M22 12h-4l-3 9L9 3l-3 9H2"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <span class="stat-label">系统健康度</span>
          </div>
          <div class="stat-value">
            <span class="stat-number">{{ stats.systemHealth }}%</span>
            <span class="stat-sublabel">正常运行时间</span>
          </div>
        </div>

        <div class="stat-card card-orange">
          <div class="stat-header">
            <svg
              class="stat-icon"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
            >
              <polyline
                points="23,4 23,10 17,10"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M20.49 15a9 9 0 11-2.12-9.36L23 10"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <span class="stat-label">总调用量</span>
          </div>
          <div class="stat-value">
            <span class="stat-number">{{ stats.totalCalls.toLocaleString() }}次</span>
            <span class="stat-sublabel">今日</span>
          </div>
        </div>
      </div>

      <!-- 已安装插件 -->
      <div class="plugins-section">
        <div class="section-header">
          <h2>已安装插件</h2>
          <button
            class="add-plugin-btn"
            @click="goToMarketplace"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
            >
              <line
                x1="12"
                y1="5"
                x2="12"
                y2="19"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
              />
              <line
                x1="5"
                y1="12"
                x2="19"
                y2="12"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
              />
            </svg>
            添加插件
          </button>
        </div>

        <div class="plugins-list">
          <div
            v-for="plugin in plugins"
            :key="plugin.id"
            class="plugin-item"
          >
            <div class="plugin-left">
              <div
                class="plugin-icon"
                :class="plugin.icon"
              >
                <svg
                  v-if="plugin.icon === 'bolt'"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <path
                    d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
                <svg
                  v-else-if="plugin.icon === 'credit'"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <rect
                    x="1"
                    y="4"
                    width="22"
                    height="16"
                    rx="2"
                    stroke="currentColor"
                    stroke-width="2"
                  />
                  <line
                    x1="1"
                    y1="10"
                    x2="23"
                    y2="10"
                    stroke="currentColor"
                    stroke-width="2"
                  />
                </svg>
                <svg
                  v-else
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <path
                    d="M22 12h-4l-3 9L9 3l-3 9H2"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </div>

              <div class="plugin-info">
                <div class="plugin-name-row">
                  <span class="plugin-name">{{ plugin.name }}</span>
                  <span class="plugin-version">v{{ plugin.version }}</span>
                </div>
                <p class="plugin-description">
                  {{ plugin.description }}
                </p>
                <div class="plugin-stats">
                  <span class="stat">
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                    >
                      <path
                        d="M22 12h-4l-3 9L9 3l-3 9H2"
                        stroke="currentColor"
                        stroke-width="2"
                      />
                    </svg>
                    {{ plugin.calls }} 次调用
                  </span>
                  <span class="stat">
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                    >
                      <circle
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        stroke-width="2"
                      />
                      <path
                        d="M12 6v6l4 2"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                      />
                    </svg>
                    {{ plugin.successRate }}% 成功率
                  </span>
                  <span class="stat">
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                    >
                      <circle
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        stroke-width="2"
                      />
                      <polyline
                        points="12,6 12,12 16,14"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                      />
                    </svg>
                    {{ plugin.latency }}ms
                  </span>
                </div>
              </div>
            </div>

            <div class="plugin-right">
              <div
                class="plugin-status"
                :class="getStatusClass(plugin.status)"
              >
                {{ getStatusLabel(plugin.status) }}
              </div>
              <label
                class="toggle"
                :class="{ 'toggle-disabled': pluginStore.isOperating(plugin.id) }"
              >
                <input
                  type="checkbox"
                  :checked="plugin.enabled"
                  :disabled="pluginStore.isOperating(plugin.id)"
                  @change="togglePlugin(plugin.id)"
                >
                <span class="toggle-slider" />
              </label>
              <!-- 操作菜单 -->
              <div class="plugin-menu-wrapper">
                <button
                  class="plugin-menu-btn"
                  @click="toggleMenu(plugin.id)"
                >
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                  >
                    <circle
                      cx="12"
                      cy="6"
                      r="1.5"
                      fill="currentColor"
                    />
                    <circle
                      cx="12"
                      cy="12"
                      r="1.5"
                      fill="currentColor"
                    />
                    <circle
                      cx="12"
                      cy="18"
                      r="1.5"
                      fill="currentColor"
                    />
                  </svg>
                </button>
                <div
                  v-if="activeMenuId === plugin.id"
                  class="plugin-menu"
                >
                  <button
                    class="menu-item"
                    @click="configurePlugin(plugin.id)"
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                    >
                      <circle
                        cx="12"
                        cy="12"
                        r="3"
                        stroke="currentColor"
                        stroke-width="2"
                      />
                      <path
                        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
                        stroke="currentColor"
                        stroke-width="2"
                      />
                    </svg>
                    配置
                  </button>
                  <button
                    class="menu-item"
                    @click="openConfirmDialog('reload', plugin.id, plugin.name)"
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                    >
                      <polyline
                        points="23,4 23,10 17,10"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                      <path
                        d="M20.49 15a9 9 0 11-2.12-9.36L23 10"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                    重载
                  </button>
                  <div class="menu-divider" />
                  <button
                    class="menu-item menu-item-danger"
                    @click="openConfirmDialog('uninstall', plugin.id, plugin.name)"
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                    >
                      <polyline
                        points="3,6 5,6 21,6"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                      <path
                        d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                    卸载
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 成功提示 Toast -->
      <Teleport to="body">
        <div
          v-if="operationSuccess"
          class="success-toast"
        >
          {{ operationSuccess }}
        </div>
      </Teleport>

      <!-- 确认对话框 -->
      <Teleport to="body">
        <div
          v-if="showConfirmDialog"
          class="confirm-overlay"
          @click="!isProcessing && closeConfirmDialog()"
        >
          <div
            class="confirm-dialog"
            @click.stop
          >
            <h3 class="confirm-title">
              {{ getConfirmTitle() }}
            </h3>
            <p class="confirm-description">
              {{ getConfirmDescription() }}
            </p>
            <!-- 错误提示 -->
            <div
              v-if="operationError"
              class="confirm-error"
            >
              {{ operationError }}
            </div>
            <div class="confirm-actions">
              <button
                class="confirm-btn confirm-btn-cancel"
                :disabled="isProcessing"
                @click="closeConfirmDialog"
              >
                取消
              </button>
              <button
                class="confirm-btn confirm-btn-confirm"
                :class="{ 'btn-danger': confirmAction === 'uninstall' }"
                :disabled="isProcessing"
                @click="executeConfirmAction"
              >
                {{ isProcessing ? '处理中...' : '确认' }}
              </button>
            </div>
          </div>
        </div>
      </Teleport>

      <!-- 配置对话框 -->
      <PluginConfigDialog
        v-if="showConfigDialog && configPluginId"
        :plugin-id="configPluginId"
        :plugin-name="configPluginName"
        :config-schema="configSchema"
        @close="closeConfigDialog"
        @saved="closeConfigDialog"
      />
    </div>
  </AppLayout>
</template>

<style scoped>
.plugins-page {
  max-width: 900px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--spacing-lg);
  margin-bottom: var(--spacing-xl);
}

.stat-card {
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  height: 100px;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.stat-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.stat-icon {
  width: 16px;
  height: 16px;
  color: var(--color-text-secondary);
  flex-shrink: 0;
}

.stat-label {
  font-size: 0.8125rem;
  color: var(--color-text-secondary);
}

/* 绿色卡片 */
.stat-card.card-green .stat-icon,
.stat-card.card-green .stat-label {
  color: var(--color-accent-green);
}

/* 橙色卡片 */
.stat-card.card-orange .stat-icon,
.stat-card.card-orange .stat-label {
  color: var(--color-accent);
}

.stat-value {
  display: flex;
  align-items: baseline;
  gap: var(--spacing-sm);
  flex-wrap: wrap;
}

.stat-number {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--color-text);
  line-height: 1;
}

.stat-change {
  font-size: 0.75rem;
  font-weight: 500;
}

.stat-change.positive {
  color: var(--color-accent-green);
}

.stat-sublabel {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
  white-space: nowrap;
}

.stat-change {
  white-space: nowrap;
}

.plugins-section {
  /* 无背景，标题和按钮直接在页面上 */
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-lg);
}

.section-header h2 {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.add-plugin-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.add-plugin-btn:hover {
  background: var(--color-bg-hover);
  border-color: var(--color-border-light);
}

.plugins-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.plugin-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--spacing-lg);
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  transition: background var(--transition-fast);
}

.plugin-item:hover {
  background: var(--color-bg-hover);
}

.plugin-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
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

.plugin-icon.bolt {
  background: var(--color-accent);
}

.plugin-icon.credit {
  background: var(--color-text-tertiary);
}

.plugin-icon.chart {
  background: var(--color-accent-blue);
}

.plugin-info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.plugin-name-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.plugin-name {
  font-weight: 600;
  color: var(--color-text);
}

.plugin-version {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
  background: var(--color-bg-tertiary);
  padding: 2px var(--spacing-sm);
  border-radius: var(--radius-sm);
}

.plugin-description {
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  margin: 0;
}

.plugin-stats {
  display: flex;
  gap: var(--spacing-lg);
  margin-top: var(--spacing-xs);
}

.plugin-stats .stat {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.plugin-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-xl);
}

.plugin-status {
  font-size: 0.75rem;
  font-weight: 500;
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: 9999px;
  white-space: nowrap;
  flex-shrink: 0;
}

.plugin-status.status-healthy {
  background: rgba(34, 197, 94, 0.15);
  color: var(--color-accent-green);
}

.plugin-status.status-degraded {
  background: rgba(239, 68, 68, 0.15);
  color: var(--color-accent-red);
}

.plugin-status.status-unhealthy {
  background: rgba(239, 68, 68, 0.25);
  color: var(--color-accent-red);
}

/* Toggle Switch */
.toggle {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
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
  border-radius: 24px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: var(--transition-fast);
  border-radius: 50%;
}

.toggle input:checked + .toggle-slider {
  background-color: var(--color-accent-green);
}

.toggle input:checked + .toggle-slider:before {
  transform: translateX(20px);
}

/* 操作菜单 */
.plugin-menu-wrapper {
  position: relative;
}

.plugin-menu-btn {
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

.plugin-menu-btn:hover {
  background: var(--color-bg-tertiary);
  color: var(--color-text);
}

.plugin-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: var(--spacing-xs);
  min-width: 120px;
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 100;
  overflow: hidden;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  width: 100%;
  padding: var(--spacing-sm) var(--spacing-md);
  background: transparent;
  border: none;
  color: var(--color-text);
  font-size: 0.875rem;
  cursor: pointer;
  transition: background var(--transition-fast);
  text-align: left;
}

.menu-item:hover {
  background: var(--color-bg-hover);
}

.menu-item-danger {
  color: var(--color-accent-red);
}

.menu-item-danger:hover {
  background: rgba(239, 68, 68, 0.1);
}

.menu-divider {
  height: 1px;
  background: var(--color-border);
  margin: var(--spacing-xs) 0;
}

/* 确认对话框 */
.confirm-overlay {
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

.confirm-dialog {
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  padding: var(--spacing-xl);
  min-width: 320px;
  max-width: 400px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
}

.confirm-title {
  margin: 0 0 var(--spacing-md) 0;
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
}

.confirm-description {
  margin: 0 0 var(--spacing-xl) 0;
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  line-height: 1.5;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
}

.confirm-btn {
  padding: var(--spacing-sm) var(--spacing-lg);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.confirm-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.confirm-btn-cancel {
  background: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  color: var(--color-text);
}

.confirm-btn-cancel:hover:not(:disabled) {
  background: var(--color-bg-hover);
}

.confirm-btn-confirm {
  background: var(--color-accent);
  border: none;
  color: white;
}

.confirm-btn-confirm:hover:not(:disabled) {
  opacity: 0.9;
}

.confirm-btn-confirm.btn-danger {
  background: var(--color-accent-red);
}

/* 错误提示 */
.confirm-error {
  margin-bottom: var(--spacing-md);
  padding: var(--spacing-sm) var(--spacing-md);
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-accent-red);
  border-radius: var(--radius-md);
  color: var(--color-accent-red);
  font-size: 0.875rem;
}

/* 成功提示 Toast */
.success-toast {
  position: fixed;
  top: var(--spacing-xl);
  right: var(--spacing-xl);
  padding: var(--spacing-md) var(--spacing-lg);
  background: var(--color-accent-green);
  color: white;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 1001;
  animation: slideIn 0.3s ease;
}

@keyframes slideIn {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

/* 禁用状态的开关 */
.toggle-disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle-disabled .toggle-slider {
  cursor: not-allowed;
}
</style>
