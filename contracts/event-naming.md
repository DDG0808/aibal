# 事件命名规范

> 版本: 1.0.0
> 冻结时间: 2025-12-27
> 状态: FROZEN

## 1. 概述

CUK 插件系统采用分层事件命名，根据事件来源使用不同的命名格式：

| 事件类型 | 命名格式 | 段数 | 说明 |
|----------|----------|------|------|
| 插件事件 | `plugin:{plugin_id}:{action}` | 三段式 | 插件之间的通信 |
| 系统事件 | `system:{action}` | 两段式 | 应用核心发出的全局事件 |
| IPC 事件 | `ipc:{action}` | 两段式 | 后端与前端的通信 |

## 2. 插件事件 (Plugin Events)

### 2.1 格式

```
plugin:{plugin_id}:{action}
```

- **plugin**: 固定前缀，表示这是插件事件
- **plugin_id**: 发布事件的插件 ID (kebab-case)
- **action**: 事件动作 (snake_case)

### 2.2 示例

| 事件名 | 说明 |
|--------|------|
| `plugin:claude-usage:data_updated` | claude-usage 插件数据更新 |
| `plugin:claude-usage:threshold_exceeded` | 使用量超过阈值 |
| `plugin:claude-usage:session_reset` | 会话重置 (使用量归零) |
| `plugin:claude-status:status_changed` | Claude 服务状态变化 |
| `plugin:notifications:sent` | 通知已发送 |

### 2.3 发布规则

```javascript
// 在插件中发布事件
// context.emit 会自动添加 "plugin:{pluginId}:" 前缀

// 插件 ID: claude-usage
context.emit('data_updated', { percentage: 75 });
// 实际发布: plugin:claude-usage:data_updated

context.emit('threshold_exceeded', { percentage: 95, threshold: 90 });
// 实际发布: plugin:claude-usage:threshold_exceeded
```

### 2.4 订阅规则

```javascript
// 在 subscribedEvents 中使用完整事件名
export const subscribedEvents = [
  'plugin:claude-usage:data_updated',
  'plugin:claude-usage:threshold_exceeded',
  'plugin:claude-status:status_changed',
];

// 在 onEvent 中处理
export async function onEvent(event, data, context) {
  switch (event) {
    case 'plugin:claude-usage:threshold_exceeded':
      await context.call('notifications', 'send', {
        title: '使用量告警',
        body: `当前使用量: ${data.percentage}%`,
      });
      break;
  }
}
```

## 3. 系统事件 (System Events)

### 3.1 格式

```
system:{action}
```

- **system**: 固定前缀，表示这是系统事件
- **action**: 系统动作 (snake_case)

### 3.2 预定义系统事件

| 事件名 | 说明 | Payload |
|--------|------|---------|
| `system:refresh_all` | 用户触发全部刷新 | `{ force?: boolean }` |
| `system:app_ready` | 应用启动完成 | `{}` |
| `system:app_will_quit` | 应用即将退出 | `{}` |
| `system:config_changed` | 应用配置变更 | `{ key: string, value: unknown }` |
| `system:network_changed` | 网络状态变化 | `{ online: boolean }` |

### 3.3 使用场景

系统事件由应用核心发出，插件可以订阅但不应发布：

```javascript
// 订阅系统事件
export const subscribedEvents = [
  'system:refresh_all',
  'system:app_will_quit',
];

export async function onEvent(event, data, context) {
  if (event === 'system:app_will_quit') {
    // 清理资源
    await cleanup(context);
  }
}
```

## 4. IPC 事件 (IPC Events)

### 4.1 格式

```
ipc:{action}
```

- **ipc**: 固定前缀，表示这是 IPC 事件
- **action**: IPC 动作 (snake_case)

### 4.2 预定义 IPC 事件

| 事件名 | 说明 | Payload 类型 |
|--------|------|-------------|
| `ipc:plugin_installed` | 插件安装完成 | `PluginInfo` |
| `ipc:plugin_uninstalled` | 插件卸载完成 | `{ id: string }` |
| `ipc:plugin_updated` | 插件更新完成 | `PluginInfo` |
| `ipc:plugin_data_updated` | 插件数据更新 | `{ id: string, data: PluginData }` |
| `ipc:plugin_error` | 插件错误 | `{ id: string, error: AppError }` |
| `ipc:plugin_health_changed` | 健康状态变化 | `PluginHealth` |

### 4.3 使用场景

IPC 事件用于 Rust 后端向 Vue 前端推送通知：

```typescript
// Vue 前端监听
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('ipc:plugin_data_updated', (event) => {
  const { id, data } = event.payload;
  updatePluginCard(id, data);
});

// 组件卸载时清理
onUnmounted(() => unlisten());
```

## 5. Action 命名约定

### 5.1 格式规则

- 使用 **snake_case** 格式
- 使用英文小写字母
- 动词优先，描述发生的事情

### 5.2 常用动作词汇

| 动作 | 含义 | 示例 |
|------|------|------|
| `data_updated` | 数据更新 | 插件获取到新数据 |
| `threshold_exceeded` | 超过阈值 | 使用量超过警戒线 |
| `session_reset` | 会话重置 | 使用量周期重置 |
| `status_changed` | 状态变化 | 服务状态改变 |
| `health_changed` | 健康变化 | 插件健康状态改变 |
| `error_occurred` | 发生错误 | 插件执行出错 |
| `config_updated` | 配置更新 | 用户修改了配置 |
| `installed` | 已安装 | 插件安装完成 |
| `uninstalled` | 已卸载 | 插件卸载完成 |
| `enabled` | 已启用 | 插件被启用 |
| `disabled` | 已禁用 | 插件被禁用 |

### 5.3 命名建议

| 推荐 | 避免 | 原因 |
|------|------|------|
| `data_updated` | `dataUpdated` | 使用 snake_case |
| `threshold_exceeded` | `threshold-exceeded` | 使用下划线分隔 |
| `status_changed` | `onStatusChange` | 不使用 on 前缀 |
| `session_reset` | `resetSession` | 使用过去式/完成式 |

## 6. 事件生命周期

### 6.1 事件流程

```
┌─────────────────────────────────────────────────────────────┐
│                       事件发布                              │
├─────────────────────────────────────────────────────────────┤
│  1. 插件调用 context.emit('action', data)                   │
│  2. 运行时添加前缀: plugin:{pluginId}:action                │
│  3. 事件进入事件总线队列                                     │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                       事件路由                              │
├─────────────────────────────────────────────────────────────┤
│  1. 遍历所有插件的 subscribedEvents                         │
│  2. 匹配订阅的事件名                                        │
│  3. 将事件加入对应插件的处理队列                             │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                       事件处理                              │
├─────────────────────────────────────────────────────────────┤
│  1. 调用插件的 onEvent(event, data, context)                │
│  2. 异步执行，不阻塞发布方                                   │
│  3. 处理错误不影响其他订阅者                                 │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 执行顺序

- 事件处理是**异步**的
- 多个订阅者的执行顺序**不保证**
- 单个订阅者的多个事件按**接收顺序**处理

## 7. 最佳实践

### 7.1 事件粒度

```javascript
// 好: 细粒度事件
context.emit('threshold_exceeded', { percentage: 95, threshold: 90 });
context.emit('session_reset', { resetTime: '2025-12-27T00:00:00Z' });

// 避免: 过于宽泛的事件
context.emit('changed', { type: 'threshold', data: {...} });
```

### 7.2 Payload 结构

```javascript
// 好: 结构清晰，包含必要信息
context.emit('threshold_exceeded', {
  percentage: 95,
  threshold: 90,
  remainingQuota: 5,
});

// 避免: 嵌套过深或包含冗余信息
context.emit('threshold_exceeded', {
  data: {
    current: { value: 95, unit: '%' },
    threshold: { value: 90, unit: '%' },
  },
});
```

### 7.3 错误处理

```javascript
export async function onEvent(event, data, context) {
  try {
    await handleEvent(event, data, context);
  } catch (error) {
    context.log('error', `Failed to handle ${event}: ${error.message}`);
    // 不抛出错误，避免影响其他事件
  }
}
```

## 8. 变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2025-12-27 | 初始冻结版本 |
