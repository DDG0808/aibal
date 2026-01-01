<script setup lang="ts">
/**
 * 应用侧边栏组件
 * Phase 8: 左侧导航栏
 */
import { ref, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';

const router = useRouter();
const route = useRoute();

// 折叠状态（从 localStorage 恢复）
const isCollapsed = ref(localStorage.getItem('sidebar-collapsed') === 'true');

// 切换折叠
function toggleCollapse() {
  isCollapsed.value = !isCollapsed.value;
  localStorage.setItem('sidebar-collapsed', String(isCollapsed.value));
}

// 菜单项
const menuItems = [
  { id: 'dashboard', label: '仪表盘', icon: 'dashboard', path: '/dashboard' },
  { id: 'plugins', label: '我的插件', icon: 'plugins', path: '/plugins' },
  { id: 'marketplace', label: '插件市场', icon: 'marketplace', path: '/marketplace' },
];

const systemItems = [
  { id: 'settings', label: '全局设置', icon: 'settings', path: '/settings' },
];

// 当前激活的菜单项
const activeItem = computed(() => route.path);

// 导航
function navigateTo(path: string) {
  router.push(path);
}

// 键盘导航处理
function handleKeydown(event: KeyboardEvent, path: string) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    navigateTo(path);
  }
}
</script>

<template>
  <aside
    class="sidebar"
    :class="{ collapsed: isCollapsed }"
  >
    <!-- macOS 原生控制按钮区域（可拖动） -->
    <div class="window-titlebar" data-tauri-drag-region />

    <!-- 应用标题 -->
    <div class="sidebar-header">
      <div class="app-logo">
        <svg
          width="40"
          height="40"
          viewBox="0 0 1024 1024"
          xmlns="http://www.w3.org/2000/svg"
          class="logo-svg"
        >
          <defs>
            <linearGradient id="sb-bgGradient" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stop-color="#1a1a2e" />
              <stop offset="100%" stop-color="#0f0f12" />
            </linearGradient>
            <radialGradient id="sb-aurora" cx="50%" cy="0%" r="80%">
              <stop offset="0%" stop-color="#10b981" stop-opacity="0.25" />
              <stop offset="50%" stop-color="#3b82f6" stop-opacity="0.1" />
              <stop offset="100%" stop-color="transparent" />
            </radialGradient>
            <linearGradient id="sb-mainGradient" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stop-color="#10b981" />
              <stop offset="50%" stop-color="#3b82f6" />
              <stop offset="100%" stop-color="#8b5cf6" />
            </linearGradient>
            <linearGradient id="sb-meterGradient" x1="0%" y1="100%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="#10b981" />
              <stop offset="60%" stop-color="#f59e0b" />
              <stop offset="100%" stop-color="#ef4444" />
            </linearGradient>
            <linearGradient id="sb-reflection" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stop-color="rgba(255,255,255,0.2)" />
              <stop offset="40%" stop-color="rgba(255,255,255,0)" />
              <stop offset="100%" stop-color="rgba(255,255,255,0.08)" />
            </linearGradient>
          </defs>
          <rect x="100" y="100" width="824" height="824" rx="280" fill="url(#sb-mainGradient)" opacity="0.35" />
          <g transform="translate(58, 58) scale(0.9)">
            <path d="M512,1024c-136,0-184.8-1.4-245.6-13.6C186.2,994,124.6,964.4,80.6,920.4s-73.6-105.6-90-185.8C-21.4,673.8-22.8,625-22.8,512s1.4-161.8,13.6-222.6C7.4,209.2,38,147.6,82,103.6s105.6-73.6,185.8-90C328.2,1.4,377,0,512,0s183.8,1.4,244.6,13.6C836.8,29.8,898.4,59.4,942.4,103.4s73.6,105.6,90,185.8C1045.4,350.2,1046.8,399,1046.8,512s-1.4,161.8-13.6,222.6c-16.4,80.2-47,141.8-91,185.8s-105.6,73.6-185.8,90C695.8,1022.6,647,1024,512,1024z" fill="url(#sb-bgGradient)" />
            <path d="M512,1024c-136,0-184.8-1.4-245.6-13.6C186.2,994,124.6,964.4,80.6,920.4s-73.6-105.6-90-185.8C-21.4,673.8-22.8,625-22.8,512s1.4-161.8,13.6-222.6C7.4,209.2,38,147.6,82,103.6s105.6-73.6,185.8-90C328.2,1.4,377,0,512,0s183.8,1.4,244.6,13.6C836.8,29.8,898.4,59.4,942.4,103.4s73.6,105.6,90,185.8C1045.4,350.2,1046.8,399,1046.8,512s-1.4,161.8-13.6,222.6c-16.4,80.2-47,141.8-91,185.8s-105.6,73.6-185.8,90C695.8,1022.6,647,1024,512,1024z" fill="url(#sb-aurora)" style="mix-blend-mode: screen;" />
          </g>
          <g transform="translate(512, 512) scale(1.25)">
            <circle cx="0" cy="0" r="260" fill="none" stroke="#ffffff" stroke-width="6" stroke-opacity="0.08" />
            <circle cx="0" cy="0" r="260" fill="none" stroke="#ffffff" stroke-width="30" stroke-opacity="0.06" stroke-dasharray="40 25" stroke-linecap="round" transform="rotate(-135)" />
            <circle cx="0" cy="0" r="260" fill="none" stroke="url(#sb-meterGradient)" stroke-width="30" stroke-dasharray="680 1000" stroke-linecap="round" transform="rotate(-135)" opacity="0.9" />
            <circle cx="0" cy="0" r="190" fill="none" stroke="url(#sb-mainGradient)" stroke-width="3" stroke-opacity="0.3" stroke-dasharray="15 10" />
            <g transform="scale(1.15)">
              <rect x="-14" y="-95" width="28" height="160" rx="14" fill="url(#sb-mainGradient)" opacity="0.9" />
              <rect x="-150" y="-105" width="300" height="24" rx="12" fill="url(#sb-mainGradient)" opacity="0.9" />
              <line x1="-120" y1="-81" x2="-120" y2="-10" stroke="url(#sb-mainGradient)" stroke-width="10" stroke-linecap="round" opacity="0.8" />
              <line x1="120" y1="-81" x2="120" y2="-30" stroke="url(#sb-mainGradient)" stroke-width="10" stroke-linecap="round" opacity="0.8" />
              <ellipse cx="-120" cy="5" rx="70" ry="22" fill="url(#sb-mainGradient)" opacity="0.7" />
              <ellipse cx="-120" cy="0" rx="58" ry="16" fill="#10b981" opacity="0.5" />
              <ellipse cx="120" cy="-15" rx="70" ry="22" fill="url(#sb-mainGradient)" opacity="0.7" />
              <ellipse cx="120" cy="-20" rx="58" ry="16" fill="#3b82f6" opacity="0.5" />
              <path d="M-45,65 L45,65 L32,90 L-32,90 Z" fill="url(#sb-mainGradient)" opacity="0.8" />
              <ellipse cx="0" cy="95" rx="50" ry="14" fill="url(#sb-mainGradient)" opacity="0.6" />
            </g>
            <circle cx="180" cy="-180" r="12" fill="#10b981" opacity="0.9" />
            <circle cx="-180" cy="180" r="10" fill="#8b5cf6" opacity="0.9" />
            <circle cx="200" cy="100" r="8" fill="#3b82f6" opacity="0.7" />
            <circle cx="-150" cy="-150" r="6" fill="#f59e0b" opacity="0.6" />
          </g>
          <g transform="translate(58, 58) scale(0.9)">
            <path d="M512,1024c-136,0-184.8-1.4-245.6-13.6C186.2,994,124.6,964.4,80.6,920.4s-73.6-105.6-90-185.8C-21.4,673.8-22.8,625-22.8,512s1.4-161.8,13.6-222.6C7.4,209.2,38,147.6,82,103.6s105.6-73.6,185.8-90C328.2,1.4,377,0,512,0s183.8,1.4,244.6,13.6C836.8,29.8,898.4,59.4,942.4,103.4s73.6,105.6,90,185.8C1045.4,350.2,1046.8,399,1046.8,512s-1.4,161.8-13.6,222.6c-16.4,80.2-47,141.8-91,185.8s-105.6,73.6-185.8,90C695.8,1022.6,647,1024,512,1024z" fill="url(#sb-reflection)" style="mix-blend-mode: overlay;" />
          </g>
        </svg>
      </div>
      <div
        v-show="!isCollapsed"
        class="app-info"
      >
        <span class="app-name">AI 监控助手</span>
        <span class="app-version">专业版</span>
      </div>
    </div>

    <!-- 主菜单 -->
    <nav
      id="sidebar-nav"
      class="sidebar-nav"
    >
      <div class="nav-section">
        <span
          v-show="!isCollapsed"
          class="nav-section-title"
        >菜单</span>
        <ul
          class="nav-list"
          role="list"
        >
          <li
            v-for="item in menuItems"
            :key="item.id"
            class="nav-item"
            :class="{ active: activeItem === item.path }"
            :title="isCollapsed ? item.label : ''"
            :aria-label="item.label"
            tabindex="0"
            role="link"
            @click="navigateTo(item.path)"
            @keydown="handleKeydown($event, item.path)"
          >
            <!-- 仪表盘图标 -->
            <svg
              v-if="item.icon === 'dashboard'"
              class="nav-icon"
              viewBox="0 0 24 24"
              fill="none"
            >
              <rect
                x="3"
                y="3"
                width="7"
                height="7"
                rx="1"
                stroke="currentColor"
                stroke-width="2"
              />
              <rect
                x="14"
                y="3"
                width="7"
                height="7"
                rx="1"
                stroke="currentColor"
                stroke-width="2"
              />
              <rect
                x="3"
                y="14"
                width="7"
                height="7"
                rx="1"
                stroke="currentColor"
                stroke-width="2"
              />
              <rect
                x="14"
                y="14"
                width="7"
                height="7"
                rx="1"
                stroke="currentColor"
                stroke-width="2"
              />
            </svg>
            <!-- 插件图标 -->
            <svg
              v-else-if="item.icon === 'plugins'"
              class="nav-icon"
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
            <!-- 市场图标 -->
            <svg
              v-else-if="item.icon === 'marketplace'"
              class="nav-icon"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9z"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <polyline
                points="9,22 9,12 15,12 15,22"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <span
              v-show="!isCollapsed"
              class="nav-label"
            >{{ item.label }}</span>
          </li>
        </ul>
      </div>

      <div class="nav-section">
        <span
          v-show="!isCollapsed"
          class="nav-section-title"
        >系统</span>
        <ul
          class="nav-list"
          role="list"
        >
          <li
            v-for="item in systemItems"
            :key="item.id"
            class="nav-item"
            :class="{ active: activeItem === item.path }"
            :title="isCollapsed ? item.label : ''"
            :aria-label="item.label"
            tabindex="0"
            role="link"
            @click="navigateTo(item.path)"
            @keydown="handleKeydown($event, item.path)"
          >
            <!-- 日志图标 -->
            <svg
              v-if="item.icon === 'logs'"
              class="nav-icon"
              viewBox="0 0 24 24"
              fill="none"
            >
              <polyline
                points="4,17 10,11 4,5"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <line
                x1="12"
                y1="19"
                x2="20"
                y2="19"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <!-- 设置图标 -->
            <svg
              v-else-if="item.icon === 'settings'"
              class="nav-icon"
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
                d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"
                stroke="currentColor"
                stroke-width="2"
              />
            </svg>
            <span
              v-show="!isCollapsed"
              class="nav-label"
            >{{ item.label }}</span>
          </li>
        </ul>
      </div>
    </nav>

    <!-- 折叠按钮 - 底部 -->
    <div class="sidebar-footer">
      <button
        class="collapse-btn"
        :title="isCollapsed ? '展开' : '折叠'"
        :aria-label="isCollapsed ? '展开侧边栏' : '折叠侧边栏'"
        :aria-expanded="!isCollapsed"
        aria-controls="sidebar-nav"
        @click="toggleCollapse"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
        >
          <path
            v-if="isCollapsed"
            d="M9 18l6-6-6-6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            v-else
            d="M15 18l-6-6 6-6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  position: relative; /* 为折叠按钮的 absolute 定位提供参考 */
  width: var(--sidebar-width);
  height: 100vh;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* macOS 原生控制按钮预留区域 */
.window-titlebar {
  height: 52px;
  -webkit-app-region: drag;
  -webkit-user-select: none;
}

.sidebar-header {
  padding: var(--spacing-lg);
  padding-top: var(--spacing-sm);
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  -webkit-app-region: no-drag; /* 确保按钮可点击 */
}

.app-logo {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.app-logo .logo-svg {
  width: 100%;
  height: 100%;
  display: block;
}

.app-info {
  display: flex;
  flex-direction: column;
}

.app-name {
  font-weight: 600;
  font-size: 0.9375rem;
  color: var(--color-text);
}

.app-version {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 0 var(--spacing-md);
  -webkit-app-region: no-drag; /* 确保导航项可点击 */
}

.nav-section {
  margin-bottom: var(--spacing-md);
}

.sidebar.collapsed .nav-section {
  margin-bottom: var(--spacing-sm);
}

.nav-section-title {
  display: block;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: var(--spacing-sm) var(--spacing-md);
}

.nav-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  padding: var(--spacing-sm) var(--spacing-md);
  margin-bottom: var(--spacing-xs);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
  color: var(--color-text-secondary);
}

.nav-item:hover,
.nav-item:focus {
  background: var(--sidebar-item-hover);
  color: var(--color-text);
  outline: none;
}

.nav-item:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: -2px;
}

.nav-item.active {
  background: var(--sidebar-item-active);
  color: var(--color-text);
  border: 1px solid var(--color-border-light);
}

.nav-icon {
  width: 20px;
  height: 20px;
  min-width: 20px;
  min-height: 20px;
  flex-shrink: 0;
}

.nav-icon svg {
  width: 100%;
  height: 100%;
}

.nav-label {
  font-size: 0.875rem;
}

/* 底部折叠按钮区域 */
.sidebar-footer {
  padding: var(--spacing-md);
  -webkit-app-region: no-drag;
}

.collapse-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.collapse-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

/* 折叠状态样式 */
.sidebar.collapsed {
  width: var(--sidebar-collapsed-width);
}

.sidebar.collapsed .sidebar-header {
  padding: var(--spacing-md);
  justify-content: center;
}

.sidebar.collapsed .app-logo {
  width: 36px;
  height: 36px;
}

.sidebar.collapsed .nav-item {
  justify-content: center;
  padding: var(--spacing-sm);
  height: 40px;
  margin-bottom: 2px;
}

.sidebar.collapsed .sidebar-nav {
  padding: 0 var(--spacing-sm);
}

/* 响应式：中等屏幕自动折叠 */
@media (max-width: 1023px) {
  .sidebar {
    width: var(--sidebar-collapsed-width);
  }

  .sidebar .app-info,
  .sidebar .nav-section-title,
  .sidebar .nav-label {
    display: none !important;
  }

  .sidebar .sidebar-header {
    padding: var(--spacing-md);
    justify-content: center;
  }

  .sidebar .app-logo {
    width: 36px;
    height: 36px;
  }

  .sidebar .collapse-btn {
    display: none;
  }

  .sidebar .nav-item {
    justify-content: center;
    padding: var(--spacing-sm);
    height: 40px;
    margin-bottom: 2px;
  }

  .sidebar .sidebar-nav {
    padding: 0 var(--spacing-sm);
  }
}

/* 响应式：小屏幕隐藏侧边栏 */
@media (max-width: 639px) {
  .sidebar {
    display: none;
  }
}
</style>
