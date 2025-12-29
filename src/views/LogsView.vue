<script setup lang="ts">
/**
 * 运行日志视图
 * Phase 8.2: 显示插件运行日志
 */
import { ref, computed } from 'vue';
import { AppLayout } from '@/components/layout';

interface LogEntry {
  id: string;
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug';
  pluginId: string;
  message: string;
}

// 过滤条件
const levelFilter = ref<string>('all');
const pluginFilter = ref<string>('all');
const searchQuery = ref('');

// 模拟日志数据
const logs = ref<LogEntry[]>([
  {
    id: '1',
    timestamp: new Date(Date.now() - 1000).toISOString(),
    level: 'info',
    pluginId: 'claude-usage',
    message: '数据刷新成功，使用量: 78%',
  },
  {
    id: '2',
    timestamp: new Date(Date.now() - 5000).toISOString(),
    level: 'info',
    pluginId: 'openai-balance',
    message: '余额查询成功，剩余: $45.23',
  },
  {
    id: '3',
    timestamp: new Date(Date.now() - 15000).toISOString(),
    level: 'warn',
    pluginId: 'deepseek-monitor',
    message: 'API 响应延迟较高: 1200ms',
  },
  {
    id: '4',
    timestamp: new Date(Date.now() - 30000).toISOString(),
    level: 'error',
    pluginId: 'deepseek-monitor',
    message: '请求超时，正在重试 (1/3)',
  },
  {
    id: '5',
    timestamp: new Date(Date.now() - 60000).toISOString(),
    level: 'debug',
    pluginId: 'claude-usage',
    message: '缓存命中，跳过网络请求',
  },
  {
    id: '6',
    timestamp: new Date(Date.now() - 120000).toISOString(),
    level: 'info',
    pluginId: 'claude-usage',
    message: '插件已启动',
  },
]);

// 可用的插件列表
const availablePlugins = computed(() => {
  const ids = new Set(logs.value.map(l => l.pluginId));
  return Array.from(ids);
});

// 过滤后的日志
const filteredLogs = computed(() => {
  return logs.value.filter(log => {
    if (levelFilter.value !== 'all' && log.level !== levelFilter.value) return false;
    if (pluginFilter.value !== 'all' && log.pluginId !== pluginFilter.value) return false;
    if (searchQuery.value && !log.message.toLowerCase().includes(searchQuery.value.toLowerCase())) return false;
    return true;
  });
});

// 格式化时间
function formatTime(isoTime: string): string {
  const date = new Date(isoTime);
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

// 清空日志
function clearLogs() {
  logs.value = [];
}

// 导出日志
function exportLogs() {
  const content = logs.value.map(l =>
    `[${formatTime(l.timestamp)}] [${l.level.toUpperCase()}] [${l.pluginId}] ${l.message}`
  ).join('\n');
  const blob = new Blob([content], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `cuk-logs-${new Date().toISOString().slice(0, 10)}.txt`;
  a.click();
  URL.revokeObjectURL(url);
}
</script>

<template>
  <AppLayout>
    <template #title>
      <h1>运行日志</h1>
    </template>

    <div class="logs-page">
      <!-- 工具栏 -->
      <div class="toolbar">
        <div class="toolbar-left">
          <div class="filter-group">
            <select v-model="levelFilter" class="filter-select">
              <option value="all">所有级别</option>
              <option value="info">信息</option>
              <option value="warn">警告</option>
              <option value="error">错误</option>
              <option value="debug">调试</option>
            </select>

            <select v-model="pluginFilter" class="filter-select">
              <option value="all">所有插件</option>
              <option v-for="id in availablePlugins" :key="id" :value="id">
                {{ id }}
              </option>
            </select>
          </div>

          <div class="search-box">
            <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none">
              <circle cx="11" cy="11" r="8" stroke="currentColor" stroke-width="2"/>
              <path d="M21 21l-4.35-4.35" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            <input
              v-model="searchQuery"
              type="text"
              placeholder="搜索日志..."
            >
          </div>
        </div>

        <div class="toolbar-right">
          <button class="toolbar-btn" @click="exportLogs">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <polyline points="7,10 12,15 17,10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <line x1="12" y1="15" x2="12" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            导出
          </button>
          <button class="toolbar-btn danger" @click="clearLogs">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <polyline points="3,6 5,6 21,6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            清空
          </button>
        </div>
      </div>

      <!-- 日志列表 -->
      <div class="logs-container">
        <div v-if="filteredLogs.length === 0" class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" stroke-width="2"/>
            <polyline points="14,2 14,8 20,8" stroke="currentColor" stroke-width="2"/>
          </svg>
          <p>暂无日志</p>
        </div>

        <div v-else class="logs-list">
          <div
            v-for="log in filteredLogs"
            :key="log.id"
            class="log-entry"
            :class="'level-' + log.level"
          >
            <span class="log-time">{{ formatTime(log.timestamp) }}</span>
            <span class="log-level">{{ log.level.toUpperCase() }}</span>
            <span class="log-plugin">{{ log.pluginId }}</span>
            <span class="log-message">{{ log.message }}</span>
          </div>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
.logs-page {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 120px);
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-lg);
  margin-bottom: var(--spacing-lg);
  flex-wrap: wrap;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.filter-group {
  display: flex;
  gap: var(--spacing-sm);
}

.filter-select {
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  color: var(--color-text);
  cursor: pointer;
}

.filter-select:focus {
  outline: none;
  border-color: var(--color-accent);
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: var(--spacing-sm);
  color: var(--color-text-tertiary);
}

.search-box input {
  padding: var(--spacing-sm) var(--spacing-md) var(--spacing-sm) 32px;
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  color: var(--color-text);
  width: 200px;
}

.search-box input::placeholder {
  color: var(--color-text-tertiary);
}

.search-box input:focus {
  outline: none;
  border-color: var(--color-accent);
}

.toolbar-right {
  display: flex;
  gap: var(--spacing-sm);
}

.toolbar-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.toolbar-btn:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.toolbar-btn.danger:hover {
  border-color: var(--color-accent-red);
  color: var(--color-accent-red);
}

.logs-container {
  flex: 1;
  background: var(--color-bg-card);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--color-text-tertiary);
  gap: var(--spacing-md);
}

.logs-list {
  font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Mono', monospace;
  font-size: 0.8125rem;
  overflow-y: auto;
  height: 100%;
  padding: var(--spacing-md);
}

.log-entry {
  display: flex;
  gap: var(--spacing-md);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-sm);
  margin-bottom: var(--spacing-xs);
  transition: background var(--transition-fast);
}

.log-entry:hover {
  background: var(--color-bg-hover);
}

.log-time {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.log-level {
  font-weight: 500;
  width: 50px;
  flex-shrink: 0;
}

.level-info .log-level {
  color: var(--color-accent-blue);
}

.level-warn .log-level {
  color: var(--color-accent-yellow);
}

.level-error .log-level {
  color: var(--color-accent-red);
}

.level-debug .log-level {
  color: var(--color-text-tertiary);
}

.log-plugin {
  color: var(--color-accent);
  flex-shrink: 0;
  width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.log-message {
  color: var(--color-text);
  flex: 1;
}
</style>
