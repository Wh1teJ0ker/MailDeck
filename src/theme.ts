/** MailDeck 主题:石墨玻璃设计语言(与图标同源)。
 *  基底为深色石墨 + 中性弥散光;表面为半透明玻璃层 + 白描边;
 *  强调色为石墨渐变,全局经 CSS 变量切换(ThemedApp 写入 :root)。 */

/** 品牌渐变(CSS 变量名,首帧默认值在 index.html 内联兜底) */
export const GRADIENT = "var(--md-gradient)";
/** 品牌主色(CSS 变量名) */
export const PRIMARY = "var(--md-primary)";
/** 渐变元素配套投影(CSS 变量名) */
export const GLOW = "var(--md-glow)";
/** 渐变悬停(跟随预设) */
export const GRADIENT_HOVER = "var(--md-gradient-hover)";

// ---------- 玻璃层 token(石墨玻璃语言,随主题不变) ----------
/** 常规玻璃面:卡片、侧栏块 */
export const GLASS = "var(--md-glass)";
/** 强玻璃面:弹窗、悬浮层 */
export const GLASS_STRONG = "var(--md-glass-strong)";
/** 玻璃描边(1px 白,低透明度) */
export const STROKE = "var(--md-stroke)";
/** 多层卡片投影 */
export const SHADOW = "var(--md-shadow)";
/** 主文字色 */
export const TEXT = "#e9ebee";
/** 次级文字色 */
export const TEXT_SUB = "#9aa3ad";
/** 页面基底(石墨,含弥散光的容器背景色) */
export const BASE = "#101318";
