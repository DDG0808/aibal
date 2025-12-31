<script setup lang="ts">
/**
 * 托盘弹窗头部组件
 * 显示插件选择器、状态、刷新按钮、主题切换按钮
 */
import { ref, computed } from 'vue';

// 简化的插件接口
interface SimplePlugin {
  id: string;
  name: string;
}

interface Props {
  /** 插件列表 */
  plugins?: SimplePlugin[];
  /** 当前选中的插件ID */
  selectedPluginId?: string;
  /** 是否正在刷新 */
  isRefreshing?: boolean;
  /** 系统状态 */
  systemStatus?: 'healthy' | 'degraded' | 'unhealthy';
  /** 是否深色模式 */
  isDarkMode?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  plugins: () => [],
  selectedPluginId: '',
  isRefreshing: false,
  systemStatus: 'healthy',
  isDarkMode: true,
});

const emit = defineEmits<{
  (e: 'select-plugin', pluginId: string): void;
  (e: 'refresh'): void;
  (e: 'toggle-theme'): void;
}>();

// 下拉菜单是否展开
const isDropdownOpen = ref(false);

// 当前选中的插件
const selectedPlugin = computed(() => {
  if (!props.selectedPluginId) return props.plugins[0];
  return props.plugins.find(p => p.id === props.selectedPluginId) || props.plugins[0];
});

// 状态指示器配置
const statusConfig = computed(() => {
  if (props.isRefreshing) {
    return { text: '处理更新', color: 'var(--color-accent-green)' };
  }
  switch (props.systemStatus) {
    case 'healthy': return { text: '运行正常', color: 'var(--color-accent-green)' };
    case 'degraded': return { text: '部分异常', color: 'var(--color-accent)' };
    case 'unhealthy': return { text: '服务异常', color: 'var(--color-accent-red)' };
    default: return { text: '运行正常', color: 'var(--color-accent-green)' };
  }
});

// 切换下拉菜单
const toggleDropdown = () => {
  if (props.plugins.length > 1) {
    isDropdownOpen.value = !isDropdownOpen.value;
  }
};

// 选择插件
const selectPlugin = (pluginId: string) => {
  emit('select-plugin', pluginId);
  isDropdownOpen.value = false;
};

// 点击外部关闭下拉
const closeDropdown = () => {
  isDropdownOpen.value = false;
};

// 刷新
const handleRefresh = () => {
  if (!props.isRefreshing) {
    emit('refresh');
  }
};

// 切换主题
const handleToggleTheme = () => {
  emit('toggle-theme');
};
</script>

<template>
  <header class="tray-header">
    <!-- 左侧：插件选择器 -->
    <div
      class="header-left"
      @click="toggleDropdown"
    >
      <!-- 插件图标 -->
      <div class="plugin-icon">
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <rect
            x="3"
            y="3"
            width="18"
            height="18"
            rx="3"
            stroke="currentColor"
            stroke-width="2"
          />
          <path
            d="M8 12h8M12 8v8"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </div>
      <div class="header-info">
        <div class="plugin-selector">
          <h1 class="plugin-name">
            {{ selectedPlugin?.name || '选择插件' }}
          </h1>
          <svg
            v-if="plugins.length > 1"
            class="dropdown-arrow"
            :class="{ 'is-open': isDropdownOpen }"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
          >
            <path
              d="M6 9l6 6 6-6"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
        <div class="status-row">
          <span
            class="status-dot"
            :style="{ backgroundColor: statusConfig.color }"
          />
          <span class="status-text">{{ statusConfig.text }}</span>
        </div>
      </div>

      <!-- 下拉菜单 -->
      <div
        v-if="isDropdownOpen && plugins.length > 1"
        class="dropdown-menu"
        @click.stop
      >
        <div
          v-for="plugin in plugins"
          :key="plugin.id"
          class="dropdown-item"
          :class="{ 'is-selected': plugin.id === selectedPluginId }"
          @click="selectPlugin(plugin.id)"
        >
          {{ plugin.name }}
        </div>
      </div>
    </div>

    <!-- 右侧：操作按钮 -->
    <div class="header-actions">
      <!-- 刷新按钮 -->
      <button
        type="button"
        class="icon-btn"
        :class="{ 'is-refreshing': isRefreshing }"
        :disabled="isRefreshing"
        :title="isRefreshing ? '正在刷新' : '刷新数据'"
        @click="handleRefresh"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M21 12a9 9 0 11-2.636-6.364"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
          <path
            d="M21 3v6h-6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>

      <!-- 明暗主题切换按钮 -->
      <button
        type="button"
        class="icon-btn"
        :title="isDarkMode ? '切换到浅色模式' : '切换到深色模式'"
        @click="handleToggleTheme"
      >
        <!-- 深色模式图标（月亮） -->
        <svg
          v-if="isDarkMode"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <!-- 浅色模式图标（太阳） -->
        <svg
          v-else
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <circle
            cx="12"
            cy="12"
            r="5"
            stroke="currentColor"
            stroke-width="2"
          />
          <path
            d="M12 1v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>

    <!-- 点击外部关闭下拉 -->
    <div
      v-if="isDropdownOpen"
      class="dropdown-backdrop"
      @click="closeDropdown"
    />
  </header>
</template>

<style scoped>
.tray-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-md);
  background: var(--color-bg-secondary);
  position: relative;
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  cursor: pointer;
  position: relative;
}

.plugin-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
}

.header-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.plugin-selector {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.plugin-name {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
  line-height: 1.2;
}

.dropdown-arrow {
  color: var(--color-text-secondary);
  transition: transform 0.2s ease;
}

.dropdown-arrow.is-open {
  transform: rotate(180deg);
}

.status-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-text {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
}

.icon-btn:hover {
  background: var(--color-bg);
  color: var(--color-text);
}

.icon-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.icon-btn.is-refreshing svg {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  min-width: 180px;
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 100;
  overflow: hidden;
  margin-top: 4px;
}

.dropdown-item {
  padding: var(--spacing-sm) var(--spacing-md);
  font-size: 0.875rem;
  color: var(--color-text);
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.dropdown-item:hover {
  background: var(--color-bg-secondary);
}

.dropdown-item.is-selected {
  background: var(--color-accent);
  color: white;
}

.dropdown-backdrop {
  position: fixed;
  inset: 0;
  z-index: 99;
}
</style>
