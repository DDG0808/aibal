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
      <svg
        width="80"
        height="80"
        viewBox="0 0 1024 1024"
        xmlns="http://www.w3.org/2000/svg"
        class="logo-image"
      >
        <defs>
          <linearGradient id="about-bgGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#1a1a2e" />
            <stop offset="100%" stop-color="#0f0f12" />
          </linearGradient>
          <radialGradient id="about-aurora" cx="50%" cy="0%" r="80%">
            <stop offset="0%" stop-color="#10b981" stop-opacity="0.25" />
            <stop offset="50%" stop-color="#3b82f6" stop-opacity="0.1" />
            <stop offset="100%" stop-color="transparent" />
          </radialGradient>
          <linearGradient id="about-mainGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#10b981" />
            <stop offset="50%" stop-color="#3b82f6" />
            <stop offset="100%" stop-color="#8b5cf6" />
          </linearGradient>
          <linearGradient id="about-meterGradient" x1="0%" y1="100%" x2="100%" y2="0%">
            <stop offset="0%" stop-color="#10b981" />
            <stop offset="60%" stop-color="#f59e0b" />
            <stop offset="100%" stop-color="#ef4444" />
          </linearGradient>
          <linearGradient id="about-reflection" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="rgba(255,255,255,0.2)" />
            <stop offset="40%" stop-color="rgba(255,255,255,0)" />
            <stop offset="100%" stop-color="rgba(255,255,255,0.08)" />
          </linearGradient>
        </defs>
        <rect x="100" y="100" width="824" height="824" rx="280" fill="url(#about-mainGradient)" opacity="0.35" />
        <g transform="translate(58, 58) scale(0.9)">
          <path d="M512,1024c-136,0-184.8-1.4-245.6-13.6C186.2,994,124.6,964.4,80.6,920.4s-73.6-105.6-90-185.8C-21.4,673.8-22.8,625-22.8,512s1.4-161.8,13.6-222.6C7.4,209.2,38,147.6,82,103.6s105.6-73.6,185.8-90C328.2,1.4,377,0,512,0s183.8,1.4,244.6,13.6C836.8,29.8,898.4,59.4,942.4,103.4s73.6,105.6,90,185.8C1045.4,350.2,1046.8,399,1046.8,512s-1.4,161.8-13.6,222.6c-16.4,80.2-47,141.8-91,185.8s-105.6,73.6-185.8,90C695.8,1022.6,647,1024,512,1024z" fill="url(#about-bgGradient)" />
          <path d="M512,1024c-136,0-184.8-1.4-245.6-13.6C186.2,994,124.6,964.4,80.6,920.4s-73.6-105.6-90-185.8C-21.4,673.8-22.8,625-22.8,512s1.4-161.8,13.6-222.6C7.4,209.2,38,147.6,82,103.6s105.6-73.6,185.8-90C328.2,1.4,377,0,512,0s183.8,1.4,244.6,13.6C836.8,29.8,898.4,59.4,942.4,103.4s73.6,105.6,90,185.8C1045.4,350.2,1046.8,399,1046.8,512s-1.4,161.8-13.6,222.6c-16.4,80.2-47,141.8-91,185.8s-105.6,73.6-185.8,90C695.8,1022.6,647,1024,512,1024z" fill="url(#about-aurora)" style="mix-blend-mode: screen;" />
        </g>
        <g transform="translate(512, 512) scale(1.25)">
          <circle cx="0" cy="0" r="260" fill="none" stroke="#ffffff" stroke-width="6" stroke-opacity="0.08" />
          <circle cx="0" cy="0" r="260" fill="none" stroke="#ffffff" stroke-width="30" stroke-opacity="0.06" stroke-dasharray="40 25" stroke-linecap="round" transform="rotate(-135)" />
          <circle cx="0" cy="0" r="260" fill="none" stroke="url(#about-meterGradient)" stroke-width="30" stroke-dasharray="680 1000" stroke-linecap="round" transform="rotate(-135)" opacity="0.9" />
          <circle cx="0" cy="0" r="190" fill="none" stroke="url(#about-mainGradient)" stroke-width="3" stroke-opacity="0.3" stroke-dasharray="15 10" />
          <g transform="scale(1.15)">
            <rect x="-14" y="-95" width="28" height="160" rx="14" fill="url(#about-mainGradient)" opacity="0.9" />
            <rect x="-150" y="-105" width="300" height="24" rx="12" fill="url(#about-mainGradient)" opacity="0.9" />
            <line x1="-120" y1="-81" x2="-120" y2="-10" stroke="url(#about-mainGradient)" stroke-width="10" stroke-linecap="round" opacity="0.8" />
            <line x1="120" y1="-81" x2="120" y2="-30" stroke="url(#about-mainGradient)" stroke-width="10" stroke-linecap="round" opacity="0.8" />
            <ellipse cx="-120" cy="5" rx="70" ry="22" fill="url(#about-mainGradient)" opacity="0.7" />
            <ellipse cx="-120" cy="0" rx="58" ry="16" fill="#10b981" opacity="0.5" />
            <ellipse cx="120" cy="-15" rx="70" ry="22" fill="url(#about-mainGradient)" opacity="0.7" />
            <ellipse cx="120" cy="-20" rx="58" ry="16" fill="#3b82f6" opacity="0.5" />
            <path d="M-45,65 L45,65 L32,90 L-32,90 Z" fill="url(#about-mainGradient)" opacity="0.8" />
            <ellipse cx="0" cy="95" rx="50" ry="14" fill="url(#about-mainGradient)" opacity="0.6" />
          </g>
          <circle cx="180" cy="-180" r="12" fill="#10b981" opacity="0.9" />
          <circle cx="-180" cy="180" r="10" fill="#8b5cf6" opacity="0.9" />
          <circle cx="200" cy="100" r="8" fill="#3b82f6" opacity="0.7" />
          <circle cx="-150" cy="-150" r="6" fill="#f59e0b" opacity="0.6" />
        </g>
        <g transform="translate(58, 58) scale(0.9)">
          <path d="M512,1024c-136,0-184.8-1.4-245.6-13.6C186.2,994,124.6,964.4,80.6,920.4s-73.6-105.6-90-185.8C-21.4,673.8-22.8,625-22.8,512s1.4-161.8,13.6-222.6C7.4,209.2,38,147.6,82,103.6s105.6-73.6,185.8-90C328.2,1.4,377,0,512,0s183.8,1.4,244.6,13.6C836.8,29.8,898.4,59.4,942.4,103.4s73.6,105.6,90,185.8C1045.4,350.2,1046.8,399,1046.8,512s-1.4,161.8-13.6,222.6c-16.4,80.2-47,141.8-91,185.8s-105.6,73.6-185.8,90C695.8,1022.6,647,1024,512,1024z" fill="url(#about-reflection)" style="mix-blend-mode: overlay;" />
        </g>
      </svg>
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

.logo-image {
  width: 80px;
  height: 80px;
  display: block;
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
