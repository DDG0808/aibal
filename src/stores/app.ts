/**
 * 应用状态管理
 * Phase 8: 管理应用全局状态
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { storageService } from '@/services/storage';
import type { AppSettings } from '@/types';
import { DEFAULT_APP_SETTINGS } from '@/types';

export const useAppStore = defineStore('app', () => {
  // 状态
  const settings = ref<AppSettings>(DEFAULT_APP_SETTINGS);
  const isInitialized = ref(false);
  const isDaemonRunning = ref(true);
  const currentRoute = ref('/dashboard');

  // 计算属性
  const refreshIntervalSeconds = computed(() => settings.value.refreshInterval / 1000);

  // 加载设置
  async function loadSettings(): Promise<void> {
    const saved = await storageService.getAppSettings();
    if (saved) {
      settings.value = { ...DEFAULT_APP_SETTINGS, ...saved };
    }
    isInitialized.value = true;
  }

  // 保存设置
  async function saveSettings(newSettings: Partial<AppSettings>): Promise<void> {
    settings.value = { ...settings.value, ...newSettings };
    await storageService.setAppSettings(settings.value);
  }

  // 设置当前路由
  function setCurrentRoute(route: string): void {
    currentRoute.value = route;
  }

  // 初始化
  async function init(): Promise<void> {
    await loadSettings();
  }

  return {
    // 状态
    settings,
    isInitialized,
    isDaemonRunning,
    currentRoute,
    // 计算属性
    refreshIntervalSeconds,
    // 方法
    loadSettings,
    saveSettings,
    setCurrentRoute,
    init,
  };
});
