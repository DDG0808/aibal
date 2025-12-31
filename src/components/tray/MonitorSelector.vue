<script setup lang="ts">
/**
 * 监控源选择器组件
 * 浮动下拉弹出层，显示所有监控源列表
 */
import { ref, computed } from 'vue';
import type { PluginInfo, PluginData } from '@/types';

interface MonitorSource {
  id: string;
  name: string;
  icon: string;
  value: string;
  type: string;
  typeLabel: string;
  status: 'healthy' | 'degraded' | 'unhealthy';
}

interface Props {
  plugins: PluginInfo[];
  pluginData: PluginData[];
  selectedId?: string;
  lastUpdated?: string;
}

const props = withDefaults(defineProps<Props>(), {
  selectedId: '',
  lastUpdated: '',
});

const emit = defineEmits<{
  (e: 'select', id: string): void;
}>();

const isExpanded = ref(false);

// 构建监控源列表
const monitorSources = computed<MonitorSource[]>(() => {
  return props.plugins
    .filter(p => p.enabled)
    .map(plugin => {
      const data = props.pluginData.find(d => d.pluginId === plugin.id);
      let value = '--';
      let type = 'unknown';
      let typeLabel = '未知';

      if (data) {
        type = data.dataType;
        if (data.dataType === 'usage') {
          const usageData = data as { percentage?: number };
          value = `${Math.round(usageData.percentage || 0)}%`;
          typeLabel = '已用';
        } else if (data.dataType === 'balance') {
          const balanceData = data as { balance?: number; currency?: string; limits?: Array<{ remaining?: number; label?: string }> };
          if (balanceData.limits && balanceData.limits.length > 0) {
            const firstLimit = balanceData.limits[0]!;
            value = typeof firstLimit.remaining === 'number'
              ? firstLimit.remaining.toFixed(2)
              : '--';
            typeLabel = firstLimit.label ?? '配额';
          } else if (typeof balanceData.balance === 'number') {
            const isCurrency = balanceData.currency === 'CNY' || balanceData.currency === 'USD' || !balanceData.currency;
            if (isCurrency) {
              value = balanceData.currency === 'CNY'
                ? `¥${balanceData.balance.toFixed(2)}`
                : `$${balanceData.balance.toFixed(2)}`;
            } else {
              value = `${balanceData.balance}`;
            }
            typeLabel = balanceData.currency && !isCurrency ? balanceData.currency : '余额';
          }
        } else if (data.dataType === 'status') {
          const statusData = data as { indicator?: string };
          value = statusData.indicator === 'none' ? '正常' : '异常';
          typeLabel = '状态';
        }
      }

      return {
        id: plugin.id,
        name: plugin.name,
        icon: plugin.icon || plugin.name.charAt(0).toUpperCase(),
        value,
        type,
        typeLabel,
        status: plugin.healthy ? 'healthy' : 'unhealthy',
      };
    });
});

const selectedSource = computed(() => {
  if (monitorSources.value.length === 0) return null;
  if (!props.selectedId) return monitorSources.value[0];
  return monitorSources.value.find(s => s.id === props.selectedId) || monitorSources.value[0];
});

const formattedLastUpdated = computed(() => {
  if (!props.lastUpdated) return '刚刚';
  try {
    const date = new Date(props.lastUpdated);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return '刚刚';
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)} 分钟前`;
    return `${Math.floor(diffSec / 3600)} 小时前`;
  } catch {
    return '刚刚';
  }
});

const toggleExpand = () => {
  isExpanded.value = !isExpanded.value;
};

const selectSource = (id: string) => {
  emit('select', id);
  isExpanded.value = false;
};

const getIconBgColor = (source: MonitorSource): string => {
  const colors: Record<string, string> = {
    usage: '#f97316',
    balance: '#22c55e',
    status: '#3b82f6',
  };
  return colors[source.type] || '#6b7280';
};
</script>

<template>
  <div class="monitor-selector">
    <!-- 主卡片头部 -->
    <div
      class="selector-header"
      @click="toggleExpand"
    >
      <div class="header-left">
        <div
          class="source-icon"
          :style="{ backgroundColor: selectedSource ? getIconBgColor(selectedSource) : '#f97316' }"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
          >
            <path
              d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
              fill="currentColor"
            />
          </svg>
        </div>
        <div class="header-info">
          <div class="header-title">
            <span class="title-text">{{ selectedSource?.name || '未选择' }}</span>
            <svg
              class="expand-icon"
              :class="{ expanded: isExpanded }"
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
          <div class="header-meta">
            <span
              class="status-badge"
              :class="selectedSource?.status"
            >
              <span class="status-dot" />
              {{ selectedSource?.status === 'healthy' ? '运行正常' : '异常' }}
            </span>
            <span class="update-time">
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
              更新于 {{ formattedLastUpdated }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- 浮动下拉弹出层 -->
    <Teleport to="body">
      <div
        v-if="isExpanded"
        class="dropdown-overlay"
        @click="isExpanded = false"
      />
      <Transition name="dropdown">
        <div
          v-if="isExpanded"
          class="dropdown-popup"
        >
          <div class="dropdown-header">
            切换监控源
          </div>
          <div class="source-list">
            <div
              v-for="source in monitorSources"
              :key="source.id"
              class="source-item"
              :class="{ selected: source.id === selectedSource?.id }"
              @click="selectSource(source.id)"
            >
              <div
                class="item-icon"
                :style="{ backgroundColor: getIconBgColor(source) }"
              >
                <svg
                  v-if="source.type === 'usage'"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <path
                    d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
                    fill="currentColor"
                  />
                </svg>
                <svg
                  v-else-if="source.type === 'balance'"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <rect
                    x="2"
                    y="6"
                    width="20"
                    height="12"
                    rx="2"
                    stroke="currentColor"
                    stroke-width="2"
                  />
                  <path
                    d="M6 12h4"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                  />
                </svg>
                <svg
                  v-else
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <path
                    d="M3 12h4l3-9 4 18 3-9h4"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </div>
              <div class="item-info">
                <span class="item-name">{{ source.name }}</span>
                <div class="item-meta">
                  <span class="item-value">{{ source.value }}</span>
                  <span class="item-type">· {{ source.typeLabel }}</span>
                </div>
              </div>
              <svg
                v-if="source.id === selectedSource?.id"
                class="check-icon"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
              >
                <path
                  d="M5 12l5 5L20 7"
                  stroke="currentColor"
                  stroke-width="2.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.monitor-selector {
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.selector-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  cursor: pointer;
  transition: background 0.2s;
}

.selector-header:hover {
  background: var(--color-bg-hover);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.source-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.header-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.title-text {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text);
}

.expand-icon {
  color: var(--color-text-secondary);
  transition: transform 0.2s;
}

.expand-icon.expanded {
  transform: rotate(180deg);
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 500;
}

.status-badge.healthy {
  background: rgba(34, 197, 94, 0.15);
  color: var(--color-accent-green);
}

.status-badge.degraded {
  background: rgba(249, 115, 22, 0.15);
  color: var(--color-accent);
}

.status-badge.unhealthy {
  background: rgba(239, 68, 68, 0.15);
  color: var(--color-accent-red);
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.update-time {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}
</style>

<style>
/* 全局样式（不使用 scoped，因为 Teleport 到 body） */
.dropdown-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9998;
}

.dropdown-popup {
  position: fixed;
  top: 80px;
  left: 16px;
  right: 16px;
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  z-index: 9999;
  overflow: hidden;
}

.dropdown-header {
  padding: 12px 16px;
  font-size: 0.8rem;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
}

.source-list {
  max-height: 280px;
  overflow-y: auto;
}

.source-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  cursor: pointer;
  transition: background 0.15s;
}

.source-item:hover {
  background: var(--color-bg-hover);
}

.source-item.selected {
  background: var(--color-bg-tertiary);
}

.item-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  flex-shrink: 0;
}

.item-info {
  flex: 1;
  min-width: 0;
}

.item-name {
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--color-text);
  display: block;
  margin-bottom: 2px;
}

.item-meta {
  display: flex;
  align-items: center;
  gap: 4px;
}

.item-value {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-accent);
}

.item-type {
  font-size: 0.75rem;
  color: var(--color-text-tertiary);
}

.check-icon {
  color: var(--color-accent);
  flex-shrink: 0;
}

/* 动画 */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
