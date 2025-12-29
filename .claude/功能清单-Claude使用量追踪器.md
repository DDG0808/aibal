# Claude 使用量追踪器 - 完整功能清单（中文版）

> 基于 hamed-elfayome/Claude-Usage-Tracker v1.5.0 源码分析
> **审核状态**: 已通过 Codex (gpt-5.2 xhigh) 审核并修正

---

## 一、项目概述

| 属性 | 描述 |
|------|------|
| **类型** | macOS 原生菜单栏应用 |
| **技术栈** | Swift 5.0+ / SwiftUI 5.0+ |
| **最低系统** | macOS 14.0 (Sonoma) |
| **架构模式** | MVVM |
| **存储方案** | UserDefaults + App Groups |
| **网络层** | URLSession (async/await) |

---

## 二、核心功能模块

### 2.1 使用量监控系统 (ClaudeAPIService)

#### 2.1.1 数据获取
| 功能点 | 描述 | API 端点 |
|--------|------|----------|
| 获取组织 ID | 从组织列表获取首个 uuid | `GET /api/organizations` |
| 获取使用量数据 | 获取完整使用统计 | `GET /api/organizations/{org_id}/usage` |
| 获取超额消费限额 | 检查 Extra 费用限制 | `GET /api/organizations/{org_id}/overage_spend_limit` |
| 创建会话 | 创建新对话 | `POST /api/organizations/{org_id}/chat_conversations` |
| 发送初始化消息 | 使用 Haiku 模型发送消息 | `POST /api/organizations/{org_id}/chat_conversations/{uuid}/completion` |

> **注意**: 初始化消息为两步操作：先创建会话，再发送 completion

#### 2.1.2 使用量数据模型 (ClaudeUsage)
| 字段 | 类型 | 描述 |
|------|------|------|
| `sessionTokensUsed` | Int | 5小时会话已用 Token |
| `sessionLimit` | Int | 5小时会话 Token 限额 |
| `sessionPercentage` | Double | 5小时会话使用百分比 |
| `sessionResetTime` | Date | 会话重置时间 |
| `weeklyTokensUsed` | Int | 周使用 Token（全模型）|
| `weeklyLimit` | Int | 周 Token 限额 |
| `weeklyPercentage` | Double | 周使用百分比 |
| `weeklyResetTime` | Date | 周重置时间 |
| `opusWeeklyTokensUsed` | Int | Opus 模型周用量 |
| `opusWeeklyPercentage` | Double | Opus 模型周使用百分比 |
| `costUsed` | Double? | Extra 已用费用 |
| `costLimit` | Double? | Extra 费用限额 |
| `costCurrency` | String? | 费用货币单位 |
| `lastUpdated` | Date | 最后更新时间 |
| `userTimezone` | TimeZone | 用户时区 |

#### 2.1.3 使用量状态级别
| 级别 | 百分比范围 | 颜色 |
|------|-----------|------|
| `safe` | 0% - 50% | 绿色 |
| `moderate` | 50% - 80% | 橙色 |
| `critical` | 80% - 100% | 红色 |

---

### 2.2 菜单栏管理器 (MenuBarManager)

#### 2.2.1 状态栏图标
- [x] 自定义菜单栏图标显示（"Claude" 文字 + 电池条）
- [x] 电池风格进度条可视化
- [x] 颜色编码状态指示（绿/橙/红）
- [x] 深色/浅色外观自适应重绘

> **注意**: 菜单栏不显示百分比数字，仅显示电池条

#### 2.2.2 弹窗管理
- [x] 点击图标弹出详细信息窗口
- [x] 弹窗可拖拽分离为独立窗口
- [x] 点击外部自动关闭弹窗
- [x] 多显示器支持

#### 2.2.3 数据刷新
- [x] 自动定时刷新（可配置间隔）
- [x] 手动刷新按钮
- [x] 并行获取使用量 + 系统状态

---

### 2.3 弹窗界面 (PopoverContentView)

#### 2.3.1 智能头部 (SmartHeader)
- [x] Logo 显示
- [x] Claude 系统状态指示器
- [x] 刷新按钮（带加载动画）

#### 2.3.2 使用量仪表盘 (SmartUsageDashboard)
- [x] 5小时会话使用量卡片
  - 使用百分比显示
  - 重置时间点显示（非倒计时）
- [x] 周使用量卡片
  - 全模型周用量
  - 百分比进度条
- [x] Opus 周使用量卡片（条件显示）
- [x] Extra 费用卡片（条件显示）
  - 已用/限额费用
  - 货币单位

> **注意**: 弹窗仅显示百分比，不显示具体 Token 数量

#### 2.3.3 上下文洞察 (ContextualInsights)
- [ ] 智能建议显示（**UI 预留，未接入**）
- [ ] 展开/收起动画（**UI 预留，未接入**）

> **注意**: `showInsights` 状态变量未被任何控件修改

#### 2.3.4 操作按钮区
- [x] 设置按钮 (gear.shape)
- [x] 退出按钮 (power)
- [x] 按钮悬停效果

---

### 2.4 通知系统 (NotificationManager)

#### 2.4.1 使用量阈值告警
| 阈值 | 百分比 | 实现状态 |
|------|--------|----------|
| `warning` | 75% | ✅ 已实现 |
| `high` | 90% | ⚠️ 常量定义但未使用 |
| `critical` | 95% | ✅ 已实现 |

> **注意**: 代码中仅实现 75% 和 95% 两个阈值，90% 为常量预留

#### 2.4.2 事件通知
- [x] 会话重置通知（0% 重置时）
- [x] 自动启动会话成功通知
- [x] macOS 原生通知集成
- [x] 前台应用时也显示通知（banner + sound）

> 前台通知由 AppDelegate 的 `userNotificationCenter(_:willPresent:)` 实现

#### 2.4.3 自动会话启动
- [x] 检测使用量从 >0% 降到 0%
- [x] 自动使用 Claude 3.5 Haiku 发送初始化消息
- [x] 可配置开关
- [x] 静默失败处理

---

### 2.5 设置界面 (SettingsView)

#### 2.5.1 侧边栏导航
| Tab | 图标 | 描述 |
|-----|------|------|
| API | `key.fill` | API 密钥配置 |
| 通用 | `gearshape` | 通用设置 |
| 会话 | `clock.arrow.circlepath` | 会话管理 |
| 通知 | `bell.badge` | 通知设置 |
| Claude Code | `terminal` | 终端集成 |
| 关于 | `info.circle` | 关于信息 |

#### 2.5.2 API 设置 (APIView)
- [x] Session Key 输入框
- [x] 密钥验证功能
- [x] 获取密钥教程链接
- [x] 密钥格式校验（sk-ant- 前缀）

#### 2.5.3 通用设置 (GeneralView)
- [x] 刷新间隔滑块（5-120秒）
- [x] 实时预览刷新间隔值
- [x] 超额消费检查开关（默认值：true）

#### 2.5.4 会话设置 (SessionView)
- [x] 自动启动会话开关
- [x] 功能说明文字

#### 2.5.5 通知设置 (NotificationsView)
- [x] 通知总开关
- [x] 请求系统通知权限
- [x] 权限状态显示

#### 2.5.6 Claude Code 设置 (StatuslineView)
- [x] 显示目录名开关
- [x] 显示 Git 分支开关
- [x] 显示使用百分比开关
- [x] 显示进度条开关
- [x] 实时预览效果
- [x] 应用配置按钮
- [x] 重置配置按钮
- [x] 状态反馈消息

#### 2.5.7 关于页面 (AboutView)
- [x] 应用 Logo
- [x] 版本号显示
- [x] 贡献者列表（从 GitHub API 获取）
- [x] GitHub 仓库链接
- [ ] 许可证信息（**UI 未实现**）

---

### 2.6 首次设置向导 (SetupWizardView)

#### 2.6.1 向导流程
- [x] 欢迎页面（Logo + 标题）
- [x] Session Key 输入区域
- [x] 获取密钥步骤说明（展开/收起）
- [x] 自动启动会话选项

#### 2.6.2 密钥验证
- [x] 格式校验（sk-ant- 前缀）
- [x] 在线连接验证
- [x] 验证状态显示（idle/validating/success/error）
- [x] 验证成功后显示组织 ID

#### 2.6.3 向导控制
- [x] 取消按钮
- [x] 验证按钮（动态文字/加载动画）
- [x] 完成按钮（验证成功后显示）

#### 2.6.4 兜底逻辑
- [x] 若 `~/.claude-session-key` 已存在，自动标记设置完成

---

### 2.7 Claude Code 终端状态栏集成 (StatuslineService)

#### 2.7.1 脚本文件
| 文件 | 路径 | 功能 |
|------|------|------|
| `fetch-claude-usage.swift` | `~/.claude/` | 获取使用量数据 |
| `statusline-command.sh` | `~/.claude/` | 构建状态栏显示 |
| `statusline-config.txt` | `~/.claude/` | 组件配置 |
| `settings.json` | `~/.claude/` | Claude Code 设置 |

#### 2.7.2 状态栏格式
```
directory │ ⎇ branch │ Usage: 25% ▓▓░░░░░░░░ → Reset: 3:45 PM
```

#### 2.7.3 可配置组件
- [x] 当前目录名
- [x] Git 分支名
- [x] 使用百分比
- [x] 10段进度条（▓░）
- [x] 重置时间点

#### 2.7.4 安装/卸载
- [x] 自动安装脚本到 ~/.claude/
- [x] 设置脚本权限（755）
- [x] 更新 settings.json
- [x] 重置/移除功能

---

### 2.8 系统状态监测 (ClaudeStatusService)

#### 2.8.1 状态级别 (ClaudeStatus)
| 级别 | 描述 | 颜色 |
|------|------|------|
| `none` | 系统正常 | 绿色 |
| `minor` | 轻微问题 | 黄色 |
| `major` | 重大故障 | 橙色 |
| `critical` | 严重故障 | 红色 |
| `unknown` | 无法获取 | 灰色 |

#### 2.8.2 状态获取
- [x] 从 statuspage.io 获取 Claude 系统状态
- [x] 10秒超时设置
- [x] 错误降级处理

---

### 2.9 数据持久化 (DataStore)

#### 2.9.1 存储键值
| 键名 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `claudeUsageData` | Data | - | 使用量数据 JSON |
| `notificationsEnabled` | Bool | true | 通知开关 |
| `refreshInterval` | Double | 30 | 刷新间隔 |
| `autoStartSessionEnabled` | Bool | false | 自动启动会话 |
| `statuslineShowDirectory` | Bool | true | 显示目录 |
| `statuslineShowBranch` | Bool | true | 显示分支 |
| `statuslineShowUsage` | Bool | true | 显示使用率 |
| `statuslineShowProgressBar` | Bool | true | 显示进度条 |
| `firstLaunchDate` | Date | - | 首次启动日期 |
| `lastGitHubStarPromptDate` | Date | - | 上次提示日期 |
| `hasStarredGitHub` | Bool | false | 是否已 Star |
| `neverShowGitHubPrompt` | Bool | false | 不再提示 |
| `hasCompletedSetup` | Bool | false | 已完成设置 |
| `checkOverageLimitEnabled` | Bool | true | 检查超额限制 |

#### 2.9.2 App Groups 支持
- [x] 共享存储标识符：`group.com.claudeusagetracker.shared`
- [x] 为未来 Widget 支持预留

---

### 2.10 GitHub Star 提示 (GitHubStarPromptView)

#### 2.10.1 显示条件
- [x] 首次启动后 1 天
- [x] 如已显示过，间隔 10 天再次提示
- [x] 用户选择"不再提示"后永不显示
- [x] 用户已 Star 后永不显示

#### 2.10.2 交互选项
- [x] "Star on GitHub" 按钮
- [x] "Maybe Later" 按钮
- [x] "Don't Ask Again" 按钮

#### 2.10.3 测试支持
- [x] 启动参数 `--show-github-prompt` 强制触发提示

---

## 三、安全特性

| 特性 | 描述 |
|------|------|
| **本地存储** | Session Key 存储在 `~/.claude-session-key` |
| **文件权限** | 密钥文件权限设置为 0600 |
| **无云同步** | 所有数据仅存储在本地 |
| **无遥测** | 零追踪、零分析 |
| **HTTPS** | 仅通过 HTTPS 与 claude.ai 通信 |
| **沙盒禁用** | 禁用沙盒以访问文件系统 |

---

## 四、应用生命周期

### 4.1 AppDelegate 职责
- [x] 禁用窗口恢复
- [x] 设置应用图标
- [x] 隐藏 Dock 图标（accessory 模式）
- [x] 请求通知权限
- [x] 检查首次设置状态
- [x] 初始化 MenuBarManager
- [x] 管理 GitHub Star 提示时机
- [x] 前台通知显示支持

### 4.2 窗口管理
- [x] 设置向导窗口
- [x] 设置窗口
- [x] GitHub 提示窗口
- [x] 分离的弹窗窗口

---

## 五、资源文件

### 5.1 图标资源 (Assets.xcassets)
| 资源 | 用途 |
|------|------|
| `AppIcon` | 应用图标（多尺寸） |
| `AboutLogo` | 关于页面 Logo |
| `HeaderLogo` | 弹窗头部 Logo |
| `WizardLogo` | 设置向导 Logo |
| `AccentColor` | 主题强调色 |

### 5.2 配置文件
| 文件 | 用途 |
|------|------|
| `Info.plist` | 应用配置 |
| `ClaudeUsageTracker.entitlements` | 权限配置 |

---

## 六、测试覆盖

### 6.1 单元测试 (Claude UsageTests)
| 测试文件 | 测试内容 |
|----------|----------|
| `ClaudeUsageTests.swift` | 使用量模型测试 |
| `DateExtensionsTests.swift` | 日期扩展测试 |
| `DataStoreTests.swift` | 数据存储测试 |

---

## 七、常量配置 (Constants)

### 7.1 刷新间隔
| 场景 | 间隔 |
|------|------|
| 菜单栏 | 30秒 |
| 小 Widget | 15分钟 |
| 中 Widget | 15分钟 |
| 大 Widget | 30分钟 |

### 7.2 通知阈值
| 级别 | 百分比 | 状态 |
|------|--------|------|
| 警告 | 75% | ✅ 已实现 |
| 高位 | 90% | ⚠️ 常量预留 |
| 临界 | 95% | ✅ 已实现 |

### 7.3 GitHub 提示时机
| 场景 | 间隔 |
|------|------|
| 首次提示延迟 | 1天 |
| 再次提示间隔 | 10天 |

---

## 八、功能统计

| 类别 | 数量 |
|------|------|
| 核心模块 | 10 |
| 视图组件 | 15+ |
| API 端点 | 5 |
| 数据存储键 | 14 |
| 设置选项 | 8 |
| 通知类型 | 4 (实际实现) |

---

## 九、未实现/预留功能

| 功能 | 位置 | 状态 |
|------|------|------|
| 上下文洞察展开/收起 | PopoverContentView | UI 预留 |
| 90% 高位通知 | NotificationManager | 常量预留 |
| 许可证信息展示 | AboutView | 未实现 |

---

**文档生成时间**: 2025-12-19
**源码版本**: v1.5.0
**分析工具**: Auggie (codebase-retrieval)
**审核工具**: Codex (gpt-5.2 + xhigh reasoning)
