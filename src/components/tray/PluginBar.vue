<script setup lang="ts">
/**
 * 插件栏组件
 * 显示已启用的插件图标和管理按钮
 */
import { computed } from 'vue';
import type { PluginInfo } from '@/types';

interface Props {
  /** 插件列表 */
  plugins: PluginInfo[];
  /** 最多显示数量 */
  maxVisible?: number;
}

const props = withDefaults(defineProps<Props>(), {
  maxVisible: 4,
});

const emit = defineEmits<{
  (e: 'manage'): void;
  (e: 'plugin-click', plugin: PluginInfo): void;
}>();

// 预定义的颜色
const pluginColors = [
  '#f97316', // orange
  '#22c55e', // green
  '#3b82f6', // blue
  '#8b5cf6', // purple
  '#ec4899', // pink
  '#14b8a6', // teal
];

// 获取插件首字母
const getInitial = (name: string): string => {
  return name.charAt(0).toUpperCase();
};

// 获取插件颜色
const getColor = (index: number): string => {
  return pluginColors[index % pluginColors.length] ?? '#22c55e';
};

// 可见插件 (响应式计算)
const visiblePlugins = computed(() => props.plugins.slice(0, props.maxVisible));
const hiddenCount = computed(() => Math.max(0, props.plugins.length - props.maxVisible));

const handlePluginClick = (plugin: PluginInfo) => {
  emit('plugin-click', plugin);
};

// 键盘事件处理
const handlePluginKeydown = (event: KeyboardEvent, plugin: PluginInfo) => {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    handlePluginClick(plugin);
  }
};

const handleManage = () => {
  emit('manage');
};

const handleManageKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    handleManage();
  }
};
</script>

<template>
  <div class="plugin-bar">
    <div class="plugin-avatars">
      <!-- 插件头像 -->
      <div
        v-for="(plugin, index) in visiblePlugins"
        :key="plugin.id"
        class="plugin-avatar"
        :style="{ backgroundColor: getColor(index) }"
        :title="plugin.name"
        :aria-label="plugin.name"
        tabindex="0"
        role="button"
        @click="handlePluginClick(plugin)"
        @keydown="handlePluginKeydown($event, plugin)"
      >
        {{ getInitial(plugin.name) }}
      </div>

      <!-- 更多指示 -->
      <div
        v-if="hiddenCount > 0"
        class="plugin-avatar more"
        :title="`还有 ${hiddenCount} 个插件`"
        :aria-label="`还有 ${hiddenCount} 个插件，点击管理`"
        tabindex="0"
        role="button"
        @click="handleManage"
        @keydown="handleManageKeydown"
      >
        ...
      </div>
    </div>

    <!-- 管理按钮 -->
    <button
      class="manage-btn"
      @click="handleManage"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <rect
          x="3"
          y="3"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
        <rect
          x="14"
          y="3"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
        <rect
          x="3"
          y="14"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
        <rect
          x="14"
          y="14"
          width="7"
          height="7"
          rx="1.5"
          stroke="currentColor"
          stroke-width="2"
        />
      </svg>
      <span>Manage Plugins</span>
    </button>
  </div>
</template>

<style scoped>
.plugin-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-md);
  border-top: 1px solid var(--color-border);
}

.plugin-avatars {
  display: flex;
  align-items: center;
  /* 重叠效果通过 margin-left 实现 */
}

.plugin-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
  color: white;
  cursor: pointer;
  transition: transform 0.2s ease;
  border: 2px solid var(--color-bg);
  margin-left: -6px;
}

.plugin-avatar:first-child {
  margin-left: 0;
}

.plugin-avatar:hover {
  transform: scale(1.1);
  z-index: 1;
}

.plugin-avatar.more {
  background: var(--color-bg-secondary);
  color: var(--color-text-secondary);
  font-size: 0.625rem;
  cursor: default;
}

.plugin-avatar.more:hover {
  transform: none;
}

.manage-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-text);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.manage-btn:hover {
  background: var(--color-bg);
  border-color: var(--color-text-secondary);
}

.manage-btn svg {
  color: var(--color-text-secondary);
}
</style>
