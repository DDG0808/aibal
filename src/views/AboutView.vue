<script setup lang="ts">
// Phase 7: 关于页面
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const version = ref('0.1.0');

onMounted(async () => {
  try {
    version.value = await invoke<string>('get_version');
  } catch (e) {
    console.error('获取版本失败:', e);
  }
});
</script>

<template>
  <div class="about-view">
    <div class="about-logo">
      <div class="logo-placeholder">
        CUK
      </div>
    </div>

    <div class="about-info">
      <h1>CUK</h1>
      <p class="description">
        Claude Usage Tracker
      </p>
      <p class="version">
        版本 {{ version }}
      </p>
    </div>

    <div class="about-details">
      <p>macOS 菜单栏应用，用于追踪 Claude AI 使用量</p>
    </div>

    <div class="about-footer">
      <p class="copyright">
        &copy; 2025 CUK Project
      </p>
      <p class="license">
        MIT License
      </p>
    </div>
  </div>
</template>

<style scoped>
.about-view {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: 24px;
  background: var(--bg-primary, #ffffff);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.about-logo {
  margin-bottom: 16px;
}

.logo-placeholder {
  width: 80px;
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 16px;
  color: white;
  font-size: 1.5rem;
  font-weight: bold;
}

.about-info {
  text-align: center;
  margin-bottom: 24px;
}

.about-info h1 {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text-primary, #333);
}

.description {
  margin: 4px 0;
  color: var(--text-secondary, #666);
}

.version {
  margin: 8px 0 0;
  font-size: 0.875rem;
  color: var(--text-tertiary, #999);
}

.about-details {
  text-align: center;
  max-width: 280px;
  margin-bottom: 24px;
}

.about-details p {
  margin: 0;
  font-size: 0.875rem;
  color: var(--text-secondary, #666);
  line-height: 1.5;
}

.about-footer {
  text-align: center;
}

.about-footer p {
  margin: 4px 0;
  font-size: 0.75rem;
  color: var(--text-tertiary, #999);
}
</style>
