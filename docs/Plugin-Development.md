# AiBal Plugin Development Guide

> Version: 1.0.0 | API Version: 1.0

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Plugin Structure](#plugin-structure)
- [Plugin Types](#plugin-types)
- [Manifest Configuration](#manifest-configuration)
- [Plugin API](#plugin-api)
- [Data Types](#data-types)
- [Event System](#event-system)
- [Cross-Plugin Communication](#cross-plugin-communication)
- [Configuration Schema](#configuration-schema)
- [Security & Permissions](#security--permissions)
- [Debugging & Testing](#debugging--testing)
- [Publishing Guide](#publishing-guide)
- [Complete Example](#complete-example)
- [FAQ](#faq)

---

## Overview

AiBal plugin system uses a three-layer architecture design, providing a secure and extensible plugin runtime environment:

```
┌─────────────────────────────────────────┐
│        Vue Frontend (Browser)            │
│  - Plugin UI Components                  │
│  - Marketplace                           │
└────────────┬────────────────────────────┘
             │ IPC Commands
             ▼
┌─────────────────────────────────────────┐
│    Tauri Backend (Rust)                  │
│  - PluginManager (Lifecycle Management)  │
│  - PluginExecutor (Sandbox Runtime)      │
│  - EventBus                              │
└────────────┬────────────────────────────┘
             │ QuickJS Sandbox
             ▼
┌─────────────────────────────────────────┐
│      Plugin Runtime (JavaScript)         │
│  - Sandboxed Execution Environment       │
│  - Secure API Injection                  │
└─────────────────────────────────────────┘
```

### Core Features

- **Sandbox Isolation**: QuickJS sandbox environment, 16MB memory limit, 30s execution timeout
- **Permission Model**: Fine-grained permission control, cross-plugin calls require explicit declaration
- **Event-Driven**: Inter-plugin communication through event bus
- **Type Safety**: Complete TypeScript type definitions
- **Hot Reload**: Support updating plugins without restarting the app

---

## Quick Start

### 1. Create Plugin Directory

```bash
mkdir my-plugin
cd my-plugin
```

### 2. Create manifest.json

```json
{
  "id": "my-plugin",
  "name": "My First Plugin",
  "version": "1.0.0",
  "apiVersion": "1.0",
  "pluginType": "data",
  "dataType": "usage",
  "author": "Your Name",
  "description": "Plugin description",
  "entry": "plugin.js",
  "refreshIntervalMs": 60000,
  "permissions": ["network"],
  "configSchema": {
    "apiKey": {
      "type": "string",
      "required": true,
      "secret": true,
      "label": "API Key"
    }
  }
}
```

### 3. Create plugin.js

```javascript
export const metadata = {
  id: 'my-plugin',
  name: 'My First Plugin',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'usage'
};

export async function fetchData(config, context) {
  return {
    dataType: 'usage',
    percentage: 50,
    used: 500,
    limit: 1000,
    unit: 'tokens',
    lastUpdated: new Date().toISOString()
  };
}
```

### 4. Install Plugin

Copy the plugin directory to `~/.config/aibal/plugins/` or install via in-app Marketplace.

---

## Plugin Structure

### Directory Structure

```
my-plugin/
├── manifest.json      # Required: Plugin manifest file
├── plugin.js          # Required: Entry file
├── icon.png           # Optional: Plugin icon (recommended 64x64)
└── assets/            # Optional: Other resource files
```

### Entry File Exports

Depending on plugin type, entry file needs to export different functions:

| Export | DataPlugin | EventPlugin | HybridPlugin |
|--------|------------|-------------|--------------|
| `metadata` | ✅ Required | ✅ Required | ✅ Required |
| `fetchData` | ✅ Required | ❌ | ✅ Required |
| `onEvent` | ❌ | ✅ Required | ✅ Required |
| `subscribedEvents` | ❌ | ✅ Required | ✅ Required |
| `exposedMethods` | ❌ | ⭕ Optional | ⭕ Optional |
| `onLoad` | ⭕ Optional | ⭕ Optional | ⭕ Optional |
| `onUnload` | ⭕ Optional | ⭕ Optional | ⭕ Optional |
| `validateConfig` | ⭕ Optional | ⭕ Optional | ⭕ Optional |

---

## Plugin Types

### DataPlugin

Used to fetch and return data, such as API usage, account balance, service status, etc.

```javascript
export const metadata = {
  id: 'usage-monitor',
  name: 'Usage Monitor',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'usage'  // usage | balance | status | custom
};

export async function fetchData(config, context) {
  const response = await fetch('https://api.example.com/usage', {
    headers: { 'Authorization': `Bearer ${config.apiKey}` }
  });

  const data = await response.json();

  return {
    dataType: 'usage',
    percentage: data.percent,
    used: data.used,
    limit: data.limit,
    unit: 'tokens',
    lastUpdated: new Date().toISOString()
  };
}
```

### EventPlugin

Used to listen and respond to events from the system or other plugins.

```javascript
export const metadata = {
  id: 'notifications',
  name: 'Notifications',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'event'
};

export const subscribedEvents = [
  'plugin:usage-monitor:threshold_exceeded',
  'system:app_ready'
];

export const exposedMethods = ['send', 'queue', 'clear'];

export async function onEvent(event, data, context) {
  if (event === 'plugin:usage-monitor:threshold_exceeded') {
    await send({
      title: 'Usage Warning',
      message: `Usage reached ${data.percentage}%`
    });
  }
}

async function send(params) {
  console.log('Notification:', params.title, params.message);
  return { success: true };
}
```

### HybridPlugin

Has both data fetching and event response capabilities.

```javascript
export const metadata = {
  id: 'smart-monitor',
  name: 'Smart Monitor',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'hybrid',
  dataType: 'usage'
};

export const subscribedEvents = [
  'system:refresh_requested'
];

export async function fetchData(config, context) {
  return { /* ... */ };
}

export async function onEvent(event, data, context) {
  // Handle events
}
```

---

## Manifest Configuration

### Complete Field Description

```json
{
  // ========== Required Fields ==========
  "id": "plugin-id",           // Unique identifier, lowercase letters, numbers, hyphens
  "name": "Plugin Name",       // Display name
  "version": "1.0.0",          // Semantic version
  "apiVersion": "1.0",         // API version (currently supports "1.0")
  "pluginType": "data",        // Plugin type: data | event | hybrid

  // ========== Required for Data Plugins ==========
  "dataType": "usage",         // Data type: usage | balance | status | custom

  // ========== Optional Fields ==========
  "author": "Author Name",
  "description": "Description",
  "homepage": "https://...",
  "icon": "icon.png",
  "entry": "plugin.js",        // Entry file (default: plugin.js)
  "refreshIntervalMs": 60000,  // Data refresh interval (ms)

  // ========== Permission Declaration ==========
  "permissions": [
    "network",                  // Network requests
    "storage",                  // Persistent storage
    "cache",                    // Memory cache
    "timer",                    // setTimeout/setInterval
    "call:notifications:send"   // Cross-plugin call
  ],

  // ========== Required for Event Plugins ==========
  "subscribedEvents": [
    "plugin:other-plugin:event_name",
    "system:app_ready"
  ],

  // ========== Exposed Methods ==========
  "exposedMethods": ["methodName"],

  // ========== Config Schema ==========
  "configSchema": {
    "fieldName": {
      "type": "string",        // string | number | boolean | select
      "required": true,
      "secret": false,
      "label": "Field Label",
      "description": "Help text",
      "default": "default value"
    }
  }
}
```

---

## Plugin API

### Context Object

Each plugin function receives a `context` object providing the following API:

```typescript
interface PluginContext {
  // ========== Read-only Properties ==========
  readonly pluginId: string;
  readonly config: Record<string, unknown>;
  readonly timeout: number;
  readonly runtimeApiVersion: string;

  // ========== Storage API (Persistent) ==========
  readonly storage: {
    get(key: string): Promise<unknown>;
    set(key: string, value: unknown): Promise<void>;
    delete(key: string): Promise<boolean>;
    keys(): Promise<string[]>;
    clear(): Promise<void>;
  };

  // ========== Cache API (Memory) ==========
  readonly cache: {
    get(key: string): Promise<unknown | null>;
    set(key: string, value: unknown, ttlMs?: number): Promise<void>;
    delete(key: string): Promise<void>;
    has(key: string): Promise<boolean>;
  };

  // ========== Methods ==========
  hasCapability(capability: string): boolean;
  log(level: 'debug' | 'info' | 'warn' | 'error', message: string): void;
  emit(event: string, data?: unknown): void;
  call(pluginId: string, method: string, params?: unknown): Promise<unknown>;
}
```

### Fetch API

Secure fetch implementation inside sandbox.

```javascript
const response = await fetch('https://api.example.com/data', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${config.apiKey}`
  },
  body: JSON.stringify({ key: 'value' })
});

if (!response.ok) {
  throw new Error(`HTTP ${response.status}`);
}

const json = response.json();
```

**Security Restrictions**:
- Forbidden access to private IPs (127.0.0.1, 192.168.*, 10.*, etc.)
- DNS resolution timeout 5 seconds
- Response body max 10MB
- Max 10 concurrent requests per plugin

---

## Data Types

### UsageData

```javascript
return {
  dataType: 'usage',
  percentage: 75,
  used: 7500,
  limit: 10000,
  unit: 'tokens',
  resetTime: '2025-01-01T00:00:00Z',
  resetLabel: 'Resets in 6 hours',
  lastUpdated: new Date().toISOString()
};
```

### BalanceData

```javascript
return {
  dataType: 'balance',
  balance: 50.00,
  currency: 'USD',
  quota: 100,
  usedQuota: 50,
  lastUpdated: new Date().toISOString()
};
```

### StatusData

```javascript
return {
  dataType: 'status',
  indicator: 'none',  // none | minor | major | critical | unknown
  description: 'All systems operational',
  lastUpdated: new Date().toISOString()
};
```

---

## Security & Permissions

### Permission Types

| Permission | Description |
|------------|-------------|
| `network` | Network requests (fetch) |
| `storage` | Persistent storage |
| `cache` | Memory cache |
| `timer` | setTimeout/setInterval |
| `call:{pluginId}:{method}` | Cross-plugin call |

### Sandbox Limits

| Limit | Value |
|-------|-------|
| Memory Limit | 16 MB |
| Stack Size | 512 KB |
| Execution Timeout | 30 seconds |
| Max Concurrent Requests | 10 |
| Response Body Size | 10 MB |
| Storage Space | 1 MB / plugin |

---

## Complete Example

### Claude API Usage Monitor Plugin

**manifest.json**:
```json
{
  "id": "claude-usage",
  "name": "Claude Usage Monitor",
  "version": "1.0.0",
  "apiVersion": "1.0",
  "pluginType": "data",
  "dataType": "usage",
  "author": "AiBal Community",
  "description": "Monitor Claude API usage",
  "refreshIntervalMs": 60000,
  "permissions": ["network", "storage"],
  "configSchema": {
    "apiKey": {
      "type": "string",
      "required": true,
      "secret": true,
      "label": "Claude API Key"
    }
  }
}
```

**plugin.js**:
```javascript
export const metadata = {
  id: 'claude-usage',
  name: 'Claude Usage Monitor',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'usage'
};

export async function fetchData(config, context) {
  context.log('debug', 'Fetching Claude API usage...');

  try {
    const response = await fetch('https://api.anthropic.com/v1/usage', {
      headers: {
        'x-api-key': config.apiKey,
        'anthropic-version': '2024-01-01'
      }
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }

    const data = response.json();
    const percentage = Math.round((data.tokens_used / data.tokens_limit) * 100);

    return {
      dataType: 'usage',
      percentage: percentage,
      used: data.tokens_used,
      limit: data.tokens_limit,
      unit: 'tokens',
      lastUpdated: new Date().toISOString()
    };

  } catch (error) {
    context.log('error', `Failed to fetch data: ${error.message}`);
    throw error;
  }
}

export async function validateConfig(config) {
  if (!config.apiKey || !config.apiKey.startsWith('sk-ant-')) {
    return {
      valid: false,
      message: 'Invalid API Key format, should start with sk-ant-'
    };
  }
  return { valid: true };
}
```

---

## FAQ

### Q: Plugin fails to load

**Possible causes**:
1. `manifest.json` format error
2. Entry file specified by `entry` doesn't exist
3. Required fields missing

**Solution**: Check app logs for specific error messages.

### Q: Fetch request fails

**Possible causes**:
1. `network` permission not declared
2. Target URL blocked by security policy
3. Network timeout

**Solution**: Add `"permissions": ["network"]` to manifest.json

### Q: Cross-plugin call fails

**Possible causes**:
1. Call permission not declared
2. Target plugin hasn't exposed the method
3. Target plugin not enabled

**Solution**: Add `"permissions": ["call:target-plugin:method-name"]`

---

*Last updated: 2025-01-01*
