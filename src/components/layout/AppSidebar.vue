<script setup lang="ts">
/**
 * 应用侧边栏组件
 * Phase 8: 左侧导航栏
 */
import { ref, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';

const router = useRouter();
const route = useRoute();

// 折叠状态
const isCollapsed = ref(false);

// 切换折叠
function toggleCollapse() {
  isCollapsed.value = !isCollapsed.value;
}

// 菜单项
const menuItems = [
  { id: 'dashboard', label: '仪表盘', icon: 'dashboard', path: '/dashboard' },
  { id: 'plugins', label: '我的插件', icon: 'plugins', path: '/plugins' },
  { id: 'marketplace', label: '插件市场', icon: 'marketplace', path: '/marketplace' },
];

const systemItems = [
  { id: 'logs', label: '运行日志', icon: 'logs', path: '/logs' },
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
    <!-- 应用标题 -->
    <div class="sidebar-header">
      <div class="app-logo">
        <svg
          width="32"
          height="32"
          viewBox="0 0 32 32"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <rect
            x="4"
            y="4"
            width="24"
            height="24"
            rx="6"
            stroke="currentColor"
            stroke-width="2"
          />
          <circle
            cx="11"
            cy="11"
            r="2"
            fill="currentColor"
          />
          <circle
            cx="16"
            cy="11"
            r="2"
            fill="currentColor"
          />
          <circle
            cx="21"
            cy="11"
            r="2"
            fill="currentColor"
          />
          <path
            d="M8 18h16M8 22h12"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </div>
      <div
        v-show="!isCollapsed"
        class="app-info"
      >
        <span class="app-name">AI 监控助手</span>
        <span class="app-version">专业版</span>
      </div>
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

    <!-- 用户信息 -->
    <div class="sidebar-footer">
      <div
        class="user-info"
        :title="isCollapsed ? '用户账户' : ''"
      >
        <div class="user-avatar">
          U
        </div>
        <div
          v-show="!isCollapsed"
          class="user-details"
        >
          <span class="user-name">用户账户</span>
          <span class="user-plan">专业版许可</span>
        </div>
        <button
          v-show="!isCollapsed"
          class="user-settings-btn"
          aria-label="用户设置"
        >
          <svg
            width="16"
            height="16"
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
        </button>
      </div>
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

.sidebar-header {
  padding: var(--spacing-lg);
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.app-logo {
  width: 40px;
  height: 40px;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text);
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

.sidebar-footer {
  padding: var(--spacing-md);
  border-top: 1px solid var(--color-border);
}

.user-info {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  padding: var(--spacing-sm);
  border-radius: var(--radius-md);
}

.user-avatar {
  width: 36px;
  height: 36px;
  background: var(--color-accent);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.875rem;
  color: white;
}

.user-details {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.user-name {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text);
}

.user-plan {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.user-settings-btn {
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.user-settings-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

/* 折叠按钮 */
.collapse-btn {
  background: none;
  border: none;
  padding: var(--spacing-sm);
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
  margin-left: auto;
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

.sidebar.collapsed .collapse-btn {
  position: absolute;
  right: var(--spacing-sm);
  top: var(--spacing-md);
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

.sidebar.collapsed .user-info {
  justify-content: center;
}

.sidebar.collapsed .user-avatar {
  width: 32px;
  height: 32px;
  font-size: 0.75rem;
}

/* 响应式：中等屏幕自动折叠 */
@media (max-width: 1023px) {
  .sidebar {
    width: var(--sidebar-collapsed-width);
  }

  .sidebar .app-info,
  .sidebar .nav-section-title,
  .sidebar .nav-label,
  .sidebar .user-details,
  .sidebar .user-settings-btn {
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

  .sidebar .user-info {
    justify-content: center;
  }

  .sidebar .user-avatar {
    width: 32px;
    height: 32px;
    font-size: 0.75rem;
  }
}

/* 响应式：小屏幕隐藏侧边栏 */
@media (max-width: 639px) {
  .sidebar {
    display: none;
  }
}
</style>
