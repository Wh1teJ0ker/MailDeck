/** MailDeck 主题:渐变蓝为默认预设,运行时通过 CSS 变量跟随设置切换。
 *  组件里使用 var(--md-gradient) / var(--md-primary) / var(--md-glow),
 *  变量值由 ThemedApp 依据 settings.accent 写入 :root。 */

/** 品牌渐变(CSS 变量名;首帧默认值在 index.html 内联,兜底蓝色) */
export const GRADIENT = "var(--md-gradient)";
/** 品牌主色(CSS 变量名) */
export const PRIMARY = "var(--md-primary)";
/** 渐变元素配套投影(CSS 变量名) */
export const GLOW = "var(--md-glow)";
/** 渐变悬停(跟随预设) */
export const GRADIENT_HOVER = "var(--md-gradient-hover)";
