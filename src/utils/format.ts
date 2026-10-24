const pad = (n: number) => String(n).padStart(2, "0");

/** 秒级 Unix 时间戳 → "YYYY-MM-DD HH:mm"(本地时区);空值给占位符。 */
export function formatTime(ts: number | null | undefined): string {
  if (!ts) return "—";
  const d = new Date(ts * 1000);
  if (Number.isNaN(d.getTime())) return "—";
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

/** 字节数 → 人类可读(B/KB/MB/GB),B 不带小数,其余一位小数。 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  return `${(mb / 1024).toFixed(1)} GB`;
}

/** 列表用发件人显示:显示名优先,回退地址,再回退占位。 */
export function formatSender(name?: string | null, addr?: string | null): string {
  const n = name?.trim();
  if (n) return n;
  if (addr) return addr;
  return "(未知发件人)";
}
