/**
 * 应用状态管理
 * Phase 8: 管理应用全局状态
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { storageService } from '@/services/storage';
import type { AppSettings, Theme } from '@/types';
import { DEFAULT_APP_SETTINGS } from '@/types';

export const useAppStore = defineStore('app', () => {
  // 状态
  const settings = ref<AppSettings>(DEFAULT_APP_SETTINGS);
  const isInitialized = ref(false);
  const isDaemonRunning = ref(true);
  const currentRoute = ref('/dashboard');

  // 实际应用的主题（解析 system 后的结果）
  const resolvedTheme = ref<'light' | 'dark'>('dark');

  // 计算属性
  const refreshIntervalSeconds = computed(() => settings.value.refreshInterval / 1000);
  const theme = computed(() => settings.value.theme);

  // 应用主题到 DOM
  function applyTheme(themeValue: Theme): void {
    const root = document.documentElement;

    // 移除现有主题类
    root.classList.remove('theme-light', 'theme-dark');

    if (themeValue === 'system') {
      // 跟随系统，不添加类让 CSS 媒体查询生效
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      resolvedTheme.value = prefersDark ? 'dark' : 'light';
    } else {
      // 手动设置主题
      root.classList.add(`theme-${themeValue}`);
      resolvedTheme.value = themeValue;
    }
  }

  // 设置主题
  async function setTheme(newTheme: Theme): Promise<void> {
    await saveSettings({ theme: newTheme });
    applyTheme(newTheme);
  }

  // 监听系统主题变化
  function setupSystemThemeListener(): void {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', (e) => {
      if (settings.value.theme === 'system') {
        resolvedTheme.value = e.matches ? 'dark' : 'light';
      }
    });
  }

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
    applyTheme(settings.value.theme);
    setupSystemThemeListener();
  }

  return {
    // 状态
    settings,
    isInitialized,
    isDaemonRunning,
    currentRoute,
    resolvedTheme,
    // 计算属性
    refreshIntervalSeconds,
    theme,
    // 方法
    loadSettings,
    saveSettings,
    setCurrentRoute,
    setTheme,
    applyTheme,
    init,
  };
});
