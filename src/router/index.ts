import { createRouter, createWebHistory } from 'vue-router';
import type { RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/dashboard',
  },
  {
    path: '/dashboard',
    name: 'dashboard',
    component: () => import('../views/DashboardView.vue'),
    meta: { title: '仪表盘 - CUK' },
  },
  {
    path: '/plugins',
    name: 'plugins',
    component: () => import('../views/PluginsView.vue'),
    meta: { title: '我的插件 - CUK' },
  },
  {
    path: '/marketplace',
    name: 'marketplace',
    component: () => import('../views/MarketplaceView.vue'),
    meta: { title: '插件市场 - CUK' },
  },
  {
    path: '/logs',
    name: 'logs',
    component: () => import('../views/LogsView.vue'),
    meta: { title: '运行日志 - CUK' },
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('../views/SettingsView.vue'),
    meta: { title: '全局设置 - CUK' },
  },
  {
    path: '/wizard',
    name: 'wizard',
    component: () => import('../views/WizardView.vue'),
    meta: { title: '欢迎使用 CUK' },
  },
  {
    path: '/about',
    name: 'about',
    component: () => import('../views/AboutView.vue'),
    meta: { title: '关于 CUK' },
  },
  // 旧路由兼容（重定向）
  {
    path: '/home',
    redirect: '/dashboard',
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// 更新页面标题
router.beforeEach((to, _from, next) => {
  document.title = (to.meta?.title as string) || 'CUK';
  next();
});

export default router;
