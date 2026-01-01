<script setup lang="ts">
/**
 * 应用主布局组件
 * Phase 8: 包含侧边栏和主内容区
 */
import { computed, onMounted, onUnmounted } from 'vue';
import { useRouter } from 'vue-router';
import AppSidebar from './AppSidebar.vue';
import { useAppStore } from '@/stores';

const appStore = useAppStore();
const router = useRouter();

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// 事件监听器清理函数
let unlistenNavigate: (() => void) | null = null;

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

// 设置导航事件监听
async function setupNavigateListener() {
  if (!isTauri) return;

  try {
    const { listen } = await import('@tauri-apps/api/event');
    unlistenNavigate = await listen<string>('navigate', (event) => {
      const route = event.payload;
      if (route && route !== router.currentRoute.value.path) {
        router.push(route);
      }
    });
  } catch (e) {
    console.warn('设置导航监听失败:', e);
  }
}

onMounted(() => {
  setupNavigateListener();
});

onUnmounted(() => {
  if (unlistenNavigate) {
    unlistenNavigate();
    unlistenNavigate = null;
  }
});
</script>

<template>
  <div class="app-layout">
    <AppSidebar />

    <main class="main-content">
      <!-- 顶部状态栏（可拖动） -->
      <header class="top-bar" data-tauri-drag-region>
        <div class="page-title">
          <slot name="title">
            <h1>页面标题</h1>
          </slot>
        </div>

        <div class="top-bar-actions">
          <!-- 官网链接 -->
          <a
            class="external-link-btn"
            href="http://devs.you/aibal"
            target="_blank"
            rel="noopener noreferrer"
            title="访问官网"
            @click.stop
          >
            <svg
              width="18"
              height="18"
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
                d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"
                stroke="currentColor"
                stroke-width="2"
              />
            </svg>
          </a>

          <!-- GitHub 链接 -->
          <a
            class="external-link-btn"
            href="https://github.com/DDG0808/aibal"
            target="_blank"
            rel="noopener noreferrer"
            title="GitHub 仓库"
            @click.stop
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path
                d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"
              />
            </svg>
          </a>

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
  padding-top: 8px !important;
  padding-bottom: 8px !important;
  background: var(--color-bg);
  border-bottom: 1px solid var(--color-border);
  min-height: 38px;
  -webkit-app-region: drag; /* 顶部栏可拖动 */
  -webkit-user-select: none;
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
  -webkit-app-region: no-drag; /* 确保按钮可点击 */
}

.external-link-btn {
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
  text-decoration: none;
}

.external-link-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-accent);
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
