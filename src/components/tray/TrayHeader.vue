<script setup lang="ts">
/**
 * 托盘弹窗头部组件
 * 显示插件选择器、运行状态指示
 */
import { ref, computed } from 'vue';

// 简化的插件接口，只需要id和name
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
  /** 系统状态 (healthy/degraded/unhealthy) */
  systemStatus?: 'healthy' | 'degraded' | 'unhealthy';
}

const props = withDefaults(defineProps<Props>(), {
  plugins: () => [],
  selectedPluginId: '',
  isRefreshing: false,
  systemStatus: 'healthy',
});

const emit = defineEmits<{
  (e: 'select-plugin', pluginId: string): void;
  (e: 'refresh'): void;
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
</script>

<template>
  <header class="tray-header">
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
            {{ selectedPlugin?.name || '选择插件' }} 配额
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

    <!-- 点击外部关闭 -->
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
  background: var(--color-bg);
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
  background: var(--color-bg-secondary);
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
