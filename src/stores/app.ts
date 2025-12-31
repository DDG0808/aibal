/**
 * 应用状态管理
 * Phase 8: 管理应用全局状态
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { storageService } from '@/services/storage';
import type { AppSettings, Theme } from '@/types';
import { DEFAULT_APP_SETTINGS } from '@/types';

// Tauri 环境检测
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// 主题同步事件名称（与 Rust 端 window/mod.rs 中的 sync_events::THEME_CHANGED 保持一致）
const THEME_CHANGED_EVENT = 'window:theme_changed';

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
    // 广播主题变化到其他窗口
    await broadcastThemeChange(newTheme);
  }

  // 广播主题变化到其他窗口
  async function broadcastThemeChange(theme: Theme): Promise<void> {
    if (!isTauri) return;
    try {
      const { emit } = await import('@tauri-apps/api/event');
      console.log('[Theme] 广播主题变化:', theme);
      await emit(THEME_CHANGED_EVENT, theme);
    } catch (e) {
      console.warn('[Theme] 广播主题变化失败:', e);
    }
  }

  // 监听其他窗口的主题变化
  async function setupThemeSyncListener(): Promise<() => void> {
    if (!isTauri) return () => {};
    try {
      const { listen } = await import('@tauri-apps/api/event');
      console.log('[Theme] 开始监听主题变化事件');
      const unlisten = await listen<Theme>(THEME_CHANGED_EVENT, async (event) => {
        const newTheme = event.payload;
        console.log('[Theme] 收到主题变化事件:', newTheme, '当前:', settings.value.theme);
        // 仅当主题与当前不同时更新（避免循环）
        if (settings.value.theme !== newTheme) {
          console.log('[Theme] 应用新主题:', newTheme);
          settings.value = { ...settings.value, theme: newTheme };
          applyTheme(newTheme);
          // 同步到存储
          await storageService.setAppSettings(settings.value);
        }
      });
      return unlisten;
    } catch (e) {
      console.warn('[Theme] 监听主题变化失败:', e);
      return () => {};
    }
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
    console.log('[AppStore] 初始化开始');
    await loadSettings();
    console.log('[AppStore] 加载设置完成, 主题:', settings.value.theme);
    applyTheme(settings.value.theme);
    setupSystemThemeListener();
    // 监听其他窗口的主题变化
    await setupThemeSyncListener();
    console.log('[AppStore] 初始化完成');
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
