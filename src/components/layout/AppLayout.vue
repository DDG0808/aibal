<script setup lang="ts">
/**
 * 应用主布局组件
 * Phase 8: 包含侧边栏和主内容区
 */
import { ref } from 'vue';
import AppSidebar from './AppSidebar.vue';

// 守护进程状态
const isDaemonRunning = ref(true);
const hasNotifications = ref(true);
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
          <!-- 守护进程状态 -->
          <div class="daemon-status" :class="{ running: isDaemonRunning }">
            <span class="status-dot"></span>
            <span class="status-text">{{ isDaemonRunning ? '守护进程运行中' : '守护进程已停止' }}</span>
          </div>

          <!-- 通知按钮 -->
          <button class="notification-btn" :class="{ 'has-unread': hasNotifications }">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M13.73 21a2 2 0 01-3.46 0" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
        </div>
      </header>

      <!-- 页面内容 -->
      <div class="page-content">
        <slot></slot>
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

.page-title h1 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.top-bar-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.daemon-status {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-secondary);
  border-radius: 9999px;
  font-size: 0.8125rem;
}

.daemon-status .status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-accent-red);
}

.daemon-status.running .status-dot {
  background: var(--color-accent-green);
}

.daemon-status .status-text {
  color: var(--color-text-secondary);
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
