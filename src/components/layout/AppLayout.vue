<script setup lang="ts">
/**
 * 应用主布局组件
 * Phase 8: 包含侧边栏和主内容区
 */
import { computed } from 'vue';
import AppSidebar from './AppSidebar.vue';
import { useAppStore } from '@/stores';

const appStore = useAppStore();

// const hasNotifications = ref(true); // 通知按钮暂时隐藏

// 主题状态：深色模式开关
const isDarkMode = computed(() => appStore.theme === 'dark' ||
  (appStore.theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches));

// 切换主题
function toggleTheme() {
  // 如果当前是 system，切换为相反的显式主题
  if (appStore.theme === 'system') {
    appStore.setTheme(isDarkMode.value ? 'light' : 'dark');
  } else {
    appStore.setTheme(appStore.theme === 'dark' ? 'light' : 'dark');
  }
}
</script>

<template>
  <div class="app-layout">
    <AppSidebar />

    <main class="main-content">
      <!-- 顶部状态栏 -->
      <header class="top-bar">
        <div class="page-title">
          <slot name="title">
            <h1>页面标题</h1>
          </slot>
        </div>

        <div class="top-bar-actions">
          <!-- 主题切换 -->
          <button
            class="theme-toggle-btn"
            :title="isDarkMode ? '切换到浅色模式' : '切换到深色模式'"
            @click="toggleTheme"
          >
            <!-- 太阳图标 (浅色模式显示) -->
            <svg
              v-if="isDarkMode"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
            >
              <circle
                cx="12"
                cy="12"
                r="4"
                stroke="currentColor"
                stroke-width="2"
              />
              <path
                d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
              />
            </svg>
            <!-- 月亮图标 (深色模式显示) -->
            <svg
              v-else
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>

          <!-- 通知按钮 (暂时隐藏)
          <button
            class="notification-btn"
            :class="{ 'has-unread': hasNotifications }"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M13.73 21a2 2 0 01-3.46 0"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
          -->
        </div>
      </header>

      <!-- 页面内容 -->
      <div class="page-content">
        <slot />
      </div>
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  height: 100vh;
  background: var(--color-bg);
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-md) var(--spacing-xl);
  background: var(--color-bg);
  border-bottom: 1px solid var(--color-border);
  min-height: 60px;
}

.page-title h1,
.page-title h2 {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
  margin: 0;
}

.top-bar-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.theme-toggle-btn {
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-secondary);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  justify-content: center;
}

.theme-toggle-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-accent);
}

.notification-btn {
  position: relative;
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-secondary);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
}

.notification-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.notification-btn.has-unread::after {
  content: '';
  position: absolute;
  top: 6px;
  right: 6px;
  width: 8px;
  height: 8px;
  background: var(--color-accent-red);
  border-radius: 50%;
}

.page-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-xl);
}
</style>
