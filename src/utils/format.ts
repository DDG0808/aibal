/**
 * 数字格式化工具
 * 用于托盘弹窗和仪表盘的统一显示
 */

/**
 * 格式化大数字（智能单位转换 K/M/B）
 */
export function formatLargeNumber(n: number): string {
  if (n === 0) return '0';
  const absN = Math.abs(n);
  if (absN >= 1_000_000_000) {
    const val = n / 1_000_000_000;
    return val % 1 === 0 ? `${val}B` : `${val.toFixed(1)}B`;
  }
  if (absN >= 1_000_000) {
    const val = n / 1_000_000;
    return val % 1 === 0 ? `${val}M` : `${val.toFixed(1)}M`;
  }
  if (absN >= 10_000) {
    const val = n / 1_000;
    return val % 1 === 0 ? `${val}K` : `${val.toFixed(1)}K`;
  }
  // 小数字保留两位小数
  if (n === Math.floor(n)) return n.toString();
  return n.toFixed(2);
}

/**
 * 配额项类型
 */
export interface QuotaDisplayItem {
  used: number;
  total: number;
  currency?: string;
}

/**
 * 判断是否为按量付费模式（quota 为 0）
 */
export function isPayAsYouGo(item: QuotaDisplayItem): boolean {
  return !item.total || item.total === 0;
}

/**
 * 格式化已用量显示
 * - 按量付费（total=0）：显示 "已用 X $"
 * - 包月（total>0）：显示 "已用 X/Y"
 */
export function formatUsedQuota(used: number, total: number, currency?: string): string {
  // 按量付费模式
  if (!total || total === 0) {
    const currencySymbol = currency || '$';
    return `已用 ${formatLargeNumber(used)} ${currencySymbol}`;
  }
  // 包月模式
  return `已用 ${formatLargeNumber(used)}/${formatLargeNumber(total)}`;
}
