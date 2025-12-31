// 88Code 订阅余额查询插件 v1.0.7
// 查询订阅套餐额度使用情况，支持订阅制和按量付费两种模式

export const metadata = {
  id: '88code-balance',
  name: '88Code 订阅余额',
  version: '1.0.7',
  apiVersion: '1.0',
  pluginType: 'data',
  dataType: 'balance',
  author: 'CUK Official',
  description: '查询 88Code 订阅套餐的额度使用情况、剩余天数和重置状态',
};

const API_URL = 'https://www.88code.org/admin-api/cc-admin/system/subscription/my';

/**
 * 判断开始时间是否已经开始
 */
function hasStarted(startDate) {
  if (!startDate) return false;
  return new Date(startDate) <= new Date();
}

/**
 * 格式化时间显示
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
 * 计算下次重置可用的状态文本
 * 仅适用于订阅制套餐
 *
 * 关键逻辑：nextResetAvailableAt 为 null 代表"现在可以刷新"
 */
function getResetStatusText(sub) {
  const { nextResetAvailableAt, resetTimes } = sub;

  // 优先判断 nextResetAvailableAt
  // 如果有值，说明需要等待到那个时间才能刷新
  if (nextResetAvailableAt) {
    return `${formatDateTime(nextResetAvailableAt)} 可重置`;
  }

  // nextResetAvailableAt 为 null，代表现在可以刷新
  // 显示剩余次数
  if (typeof resetTimes === 'number' && resetTimes < 2) {
    return `可重置 (${2 - resetTimes}/2)`;
  }

  return '可重置';
}

/**
 * 判断是否为按量付费模式
 * 支持多种可能的 planType 值和套餐名称
 */
function isPayPerUse(sub) {
  const plan = sub.subscriptionPlan || {};
  const planType = (plan.planType || '').toUpperCase();
  const planName = (sub.subscriptionPlanName || '').toUpperCase();

  // 检查 planType
  if (planType === 'PAY_PER_USE' || planType === 'PAYGO' || planType === 'PAY_AS_YOU_GO') {
    return true;
  }

  // 检查套餐名称是否包含按量付费相关关键词
  if (planName.includes('PAYGO') || planName.includes('按量') || planName.includes('PAY_PER_USE')) {
    return true;
  }

  // 检查 creditLimit 是否为 0 或不存在（按量付费通常没有固定额度上限）
  // 但有 currentCredits（有余额）
  const creditLimit = plan.creditLimit;
  const currentCredits = sub.currentCredits;
  if ((creditLimit === 0 || creditLimit === null || creditLimit === undefined) &&
      typeof currentCredits === 'number' && currentCredits >= 0) {
    // 额外检查：如果没有重置功能也可能是按量付费
    if (!sub.canResetNow && !sub.nextResetAvailableAt) {
      return true;
    }
  }

  return false;
}

/**
 * 获取 88Code 订阅余额数据
 */
export async function fetchData(config, context) {
  const { authToken } = config;

  if (!authToken) {
    throw new Error('请配置 Authorization Token');
  }

  let token = authToken.trim();

  try {
    context.log('info', `正在请求 88Code API...`);

    const response = await fetch(API_URL, {
      method: 'GET',
      headers: {
        'Authorization': token,
      },
    });

    const responseText = await response.text();

    if (!response.ok) {
      let errorMsg = `HTTP ${response.status}`;
      try {
        const errorData = JSON.parse(responseText);
        if (errorData.msg) {
          errorMsg = errorData.msg;
        }
      } catch (e) {
        if (responseText.length < 200) {
          errorMsg += `: ${responseText}`;
        }
      }

      if (response.status === 403) {
        throw new Error(`认证失败 (403): Token 可能已过期或格式不正确`);
      } else if (response.status === 401) {
        throw new Error(`未授权 (401): 请检查 Token 是否正确`);
      }

      throw new Error(errorMsg);
    }

    let result;
    try {
      result = JSON.parse(responseText);
    } catch (e) {
      throw new Error(`响应解析失败: ${responseText.substring(0, 100)}`);
    }

    if (result.code !== 0 || !result.ok) {
      throw new Error(result.msg || `API 错误: ${result.code}`);
    }

    const subscriptions = result.data || [];

    if (subscriptions.length === 0) {
      throw new Error('未找到订阅数据');
    }

    // 筛选出活跃中且已开始的订阅
    const activeSubscriptions = subscriptions.filter(sub => {
      const isActive = sub.subscriptionStatus === '活跃中' || sub.isActive === true;
      const started = hasStarted(sub.startDate);
      return isActive && started;
    });

    if (activeSubscriptions.length === 0) {
      const firstSub = subscriptions[0];
      throw new Error(`没有活跃的订阅 (状态: ${firstSub.subscriptionStatus || '未知'})`);
    }

    // 转换为 limits 数组显示
    const limits = activeSubscriptions.map(sub => {
      const plan = sub.subscriptionPlan || {};
      const currentCredits = sub.currentCredits || 0;  // 剩余额度
      const payPerUse = isPayPerUse(sub);

      context.log('info', `订阅 ${sub.subscriptionPlanName}: planType=${plan.planType}, creditLimit=${plan.creditLimit}, isPayPerUse=${payPerUse}, nextResetAvailableAt=${sub.nextResetAvailableAt}`);

      // 按量付费模式：只显示剩余金额，不显示重置信息
      if (payPerUse) {
        // 构建状态文本：仅显示剩余天数，无重置信息
        let statusText = `剩余 ${sub.remainingDays} 天`;

        return {
          limitType: sub.subscriptionPlanName || 'PAY_PER_USE',
          label: sub.subscriptionPlanName || '按量付费',
          unit: '$',
          usage: null,                 // 按量付费无总额度概念
          remaining: currentCredits,   // 剩余金额
          currentValue: null,          // 按量付费不计算已用
          percentage: null,            // 按量付费不显示百分比
          nextResetTime: null,         // 无重置功能
          statusText: statusText,
          resetToday: null,            // 无重置功能，设为 null 而不是 false
          isPayPerUse: true,           // 标记为按量付费
        };
      }

      // 订阅制：显示完整的额度和重置信息
      const creditLimit = plan.creditLimit || 0;  // 总额度
      const used = creditLimit - currentCredits;  // 已用额度
      // 使用百分比 = 已用 / 总额度 * 100
      const percentage = creditLimit > 0 ? Math.round((used / creditLimit) * 100) : 0;

      // 构建状态文本
      let statusText = `剩余 ${sub.remainingDays} 天`;
      const resetStatus = getResetStatusText(sub);
      if (resetStatus) {
        statusText += ` | ${resetStatus}`;
      }

      // 修复: nextResetAvailableAt 为 null 代表可以刷新
      // nextResetAvailableAt 有值代表今天已重置过，需要等待
      const resetToday = sub.nextResetAvailableAt != null;

      return {
        limitType: sub.subscriptionPlanName || 'SUBSCRIPTION',
        label: sub.subscriptionPlanName || '订阅套餐',
        unit: '$',
        usage: creditLimit,           // 总额度
        remaining: currentCredits,    // 剩余额度
        currentValue: used > 0 ? used : 0,  // 已用额度
        percentage: percentage,       // 使用百分比
        nextResetTime: sub.nextResetAvailableAt || null,
        statusText: statusText,
        resetToday: resetToday,       // 修复: 基于 nextResetAvailableAt 判断
        isPayPerUse: false,           // 标记为订阅制
      };
    });

    // 计算总剩余余额
    const totalBalance = activeSubscriptions.reduce((sum, sub) => sum + (sub.currentCredits || 0), 0);

    context.log('info', `成功获取 ${activeSubscriptions.length} 个活跃订阅`);

    return {
      dataType: 'balance',
      balance: Math.round(totalBalance * 100) / 100,
      currency: 'USD',
      limits: limits,
      showTotal: false,
      lastUpdated: new Date().toISOString(),
    };
  } catch (error) {
    context.log('error', `获取 88Code 余额失败: ${error.message}`);
    throw error;
  }
}

export async function onLoad(context) {
  context.log('info', '88Code 订阅余额插件 v1.0.7 已加载');
}

export async function onUnload(context) {
  context.log('info', '88Code 订阅余额插件已卸载');
}

export async function validateConfig(config) {
  if (!config.authToken) {
    return { valid: false, message: 'Authorization Token 不能为空' };
  }
  if (config.authToken.length < 10) {
    return { valid: false, message: 'Token 格式不正确（太短）' };
  }
  return { valid: true };
}
