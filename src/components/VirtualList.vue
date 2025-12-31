<script setup lang="ts" generic="T">
/**
 * 虚拟滚动列表组件
 * 适用于大量数据的高性能渲染
 */
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';

const props = withDefaults(defineProps<{
  items: T[];
  itemHeight: number;
  bufferSize?: number;
  /** 获取列表项唯一键值，避免使用 index 导致的 DOM 复用问题 */
  getKey?: (item: T, index: number) => string | number;
}>(), {
  bufferSize: 5,
});

defineSlots<{
  default(props: { item: T; index: number }): unknown;
}>();

const containerRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(0);

// 总高度
const totalHeight = computed(() => props.items.length * props.itemHeight);

// 可见范围
const visibleRange = computed(() => {
  const start = Math.max(0, Math.floor(scrollTop.value / props.itemHeight) - props.bufferSize);
  const visibleCount = Math.ceil(containerHeight.value / props.itemHeight) + props.bufferSize * 2;
  const end = Math.min(props.items.length, start + visibleCount);
  return { start, end };
});

// 可见项目
const visibleItems = computed(() => {
  const { start, end } = visibleRange.value;
  return props.items.slice(start, end).map((item, index) => ({
    item,
    index: start + index,
  }));
});

// 偏移量
const offsetTop = computed(() => visibleRange.value.start * props.itemHeight);

// 滚动处理
function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  scrollTop.value = target.scrollTop;
}

// 更新容器高度
function updateContainerHeight() {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight;
  }
}

// 监听容器大小变化
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  updateContainerHeight();
  if (containerRef.value) {
    resizeObserver = new ResizeObserver(updateContainerHeight);
    resizeObserver.observe(containerRef.value);
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
});

// 当 items 改变时智能处理滚动位置
watch(() => props.items.length, (newLength, oldLength) => {
  if (!containerRef.value) return;

  const isAppend = newLength > (oldLength ?? 0) && oldLength !== undefined;

  if (isAppend) {
    // 追加模式：如果用户已接近底部，自动滚动到底部
    const container = containerRef.value;
    const isNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 100;
    if (isNearBottom) {
      // 使用 requestAnimationFrame 确保 DOM 已更新
      requestAnimationFrame(() => {
        if (containerRef.value) {
          containerRef.value.scrollTop = containerRef.value.scrollHeight;
          scrollTop.value = containerRef.value.scrollTop;
        }
      });
    }
    // 不在底部时保持当前位置
  } else {
    // 替换模式：重置到顶部
    containerRef.value.scrollTop = 0;
    scrollTop.value = 0;
  }
});
</script>

<template>
  <div
    ref="containerRef"
    class="virtual-list-container"
    @scroll="handleScroll"
  >
    <div
      class="virtual-list-spacer"
      :style="{ height: totalHeight + 'px' }"
    >
      <div
        class="virtual-list-content"
        :style="{ transform: `translateY(${offsetTop}px)` }"
      >
        <div
          v-for="{ item, index } in visibleItems"
          :key="getKey ? getKey(item, index) : index"
          class="virtual-list-item"
          :style="{ height: itemHeight + 'px' }"
        >
          <slot
            :item="item"
            :index="index"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.virtual-list-container {
  overflow-y: auto;
  height: 100%;
}

.virtual-list-spacer {
  position: relative;
}

.virtual-list-content {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
}

.virtual-list-item {
  overflow: hidden;
}
</style>
