// Antigravity 配额查询插件 v1.0.0
// 查询 Google Cloud Code 各模型的配额使用情况

export const metadata = {
  id: 'antigravity',
  name: 'Antigravity 配额',
  version: '1.0.0',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'balance',
  author: 'CUK Official',
  description: '查询 Google Cloud Code 各模型的配额使用情况和重置时间',
};

const API_URL = 'https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:fetchAvailableModels';

/**
 * 格式化重置时间显示
 */
function formatResetTime(resetTime) {
  if (!resetTime) return null;

  const reset = new Date(resetTime);
  const now = new Date();
  const diffMs = reset.getTime() - now.getTime();

  if (diffMs <= 0) return '即将重置';

  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffMinutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));

  if (diffHours > 24) {
    const days = Math.floor(diffHours / 24);
    return `${days}天后重置`;
  } else if (diffHours > 0) {
    return `${diffHours}小时${diffMinutes}分后重置`;
  } else {
    return `${diffMinutes}分钟后重置`;
  }
}

/**
 * 格式化时间为本地格式
 */
function formatDateTime(dateStr) {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  return `${month}-${day} ${hours}:${minutes}`;
}

/**
 * 获取模型配额数据
 */
export async function fetchData(config, context) {
  const { authorization, userAgent, clientSecret, clientId } = config;

  if (!authorization) {
    throw new Error('请配置 Authorization');
  }
  if (!userAgent) {
    throw new Error('请配置 User-Agent');
  }
  if (!clientSecret) {
    throw new Error('请配置 Client Secret');
  }
  if (!clientId) {
    throw new Error('请配置 Client ID');
  }

  try {
    context.log('info', '正在请求 Antigravity API...');

    const response = await fetch(API_URL, {
      method: 'POST',
      headers: {
        'Authorization': authorization.trim(),
        'Content-Type': 'application/json',
        'User-Agent': userAgent.trim(),
        'client_secret': clientSecret.trim(),
        'client_id': clientId.trim(),
      },
      body: '{}',
    });

    const responseText = await response.text();

    if (!response.ok) {
      let errorMsg = `HTTP ${response.status}`;
      try {
        const errorData = JSON.parse(responseText);
        if (errorData.error && errorData.error.message) {
          errorMsg = errorData.error.message;
        }
      } catch (e) {
        if (responseText.length < 200) {
          errorMsg += `: ${responseText}`;
        }
      }

      if (response.status === 403) {
        throw new Error(`认证失败 (403): Token 可能已过期`);
      } else if (response.status === 401) {
        throw new Error(`未授权 (401): 请检查认证信息`);
      }

      throw new Error(errorMsg);
    }

    let result;
    try {
      result = JSON.parse(responseText);
    } catch (e) {
      throw new Error(`响应解析失败: ${responseText.substring(0, 100)}`);
    }

    const models = result.models || {};

    if (Object.keys(models).length === 0) {
      throw new Error('未找到模型数据');
    }

    // 筛选有 displayName 的模型（有效套餐）
    const validModels = Object.entries(models)
      .filter(([_, model]) => model.displayName)
      .map(([id, model]) => ({
        id,
        ...model,
      }));

    if (validModels.length === 0) {
      throw new Error('未找到有效的模型套餐');
    }

    context.log('info', `找到 ${validModels.length} 个有效模型套餐`);

    // 按剩余配额排序（从低到高，方便用户关注即将用尽的）
    validModels.sort((a, b) => {
      const aRemaining = a.quotaInfo?.remainingFraction ?? 1;
      const bRemaining = b.quotaInfo?.remainingFraction ?? 1;
      return aRemaining - bRemaining;
    });

    // 转换为 limits 数组
    const limits = validModels.map(model => {
      const quotaInfo = model.quotaInfo || {};
      const remainingFraction = quotaInfo.remainingFraction ?? 1;
      const resetTime = quotaInfo.resetTime || null;

      // 计算使用百分比（已用 = 1 - 剩余）
      const usedFraction = 1 - remainingFraction;
      const percentage = Math.round(usedFraction * 100);

      // 构建状态文本
      let statusText = `剩余 ${Math.round(remainingFraction * 100)}%`;
      if (resetTime) {
        const resetLabel = formatResetTime(resetTime);
        if (resetLabel) {
          statusText += ` | ${resetLabel}`;
        }
      }

      return {
        limitType: model.id,
        label: model.displayName,
        unit: '%',
        usage: 100,                           // 总额度 100%
        remaining: Math.round(remainingFraction * 100),  // 剩余百分比
        currentValue: Math.round(usedFraction * 100),    // 已用百分比
        percentage: percentage,               // 使用百分比进度条
        nextResetTime: resetTime,
        statusText: statusText,
        resetToday: false,
        isPayPerUse: false,
      };
    });

    // 计算平均剩余率作为总体状态
    const avgRemaining = validModels.reduce((sum, m) =>
      sum + (m.quotaInfo?.remainingFraction ?? 1), 0) / validModels.length;

    context.log('info', `成功获取 ${validModels.length} 个模型配额，平均剩余 ${Math.round(avgRemaining * 100)}%`);

    return {
      dataType: 'balance',
      balance: Math.round(avgRemaining * 100),  // 平均剩余百分比
      currency: '%',
      limits: limits,
      showTotal: false,
      lastUpdated: new Date().toISOString(),
    };
  } catch (error) {
    context.log('error', `获取配额失败: ${error.message}`);
    throw error;
  }
}

export async function onLoad(context) {
  context.log('info', 'Antigravity 配额插件 v1.0.0 已加载');
}

export async function onUnload(context) {
  context.log('info', 'Antigravity 配额插件已卸载');
}

export async function validateConfig(config) {
  if (!config.authorization) {
    return { valid: false, message: 'Authorization 不能为空' };
  }
  if (!config.userAgent) {
    return { valid: false, message: 'User-Agent 不能为空' };
  }
  if (!config.clientSecret) {
    return { valid: false, message: 'Client Secret 不能为空' };
  }
  if (!config.clientId) {
    return { valid: false, message: 'Client ID 不能为空' };
  }
  return { valid: true };
}
