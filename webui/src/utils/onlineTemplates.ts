import { parse as parseToml } from 'smol-toml'
import type {
  OnlineTemplateDetail,
  OnlineTemplateIndexItem,
  OnlineTemplateSource,
  Template,
  TemplateCategory,
  TemplateMeta,
} from '../types'
import { execCommand } from './ksu'
import { extractTemplateMeta, sanitizeTemplate } from './config'

const REQUEST_TIMEOUT_MS = 15000
const SHELL_TIMEOUT_SECONDS = Math.ceil(REQUEST_TIMEOUT_MS / 1000)
const DETAIL_CONCURRENCY = 6
/** 聚合文件候选链整体重试次数（第二次尝试前等 TEMPLATE_BUNDLE_RETRY_DELAY_MS）。 */
const TEMPLATE_BUNDLE_ATTEMPTS = 2
const TEMPLATE_BUNDLE_RETRY_DELAY_MS = 400
const FETCH_ACCEPT_TEXT = 'text/plain'
const FETCH_ACCEPT_JSON = 'application/json'
const FETCH_ACCEPT_GITHUB_JSON = 'application/vnd.github+json'

const SOURCE_CONFIGS = {
  gitee: {
    owner: 'Seyud',
    repo: 'device_faker_config_mirror',
    apiBase: 'https://gitee.com/api/v5',
    rawBase: 'https://gitee.com/Seyud/device_faker_config_mirror/raw/main',
    cdnBase: 'https://cdn.jsdelivr.net/gh/Seyud/device_faker_config@main',
  },
  github: {
    owner: 'Seyud',
    repo: 'device_faker_config',
    apiBase: 'https://api.github.com',
    rawBase: 'https://raw.githubusercontent.com/Seyud/device_faker_config/main',
    cdnBase: 'https://cdn.jsdelivr.net/gh/Seyud/device_faker_config@main',
  },
} as const satisfies Record<
  OnlineTemplateSource,
  {
    owner: string
    repo: string
    apiBase: string
    rawBase: string
    cdnBase: string
  }
>

export const TEMPLATE_CATEGORIES = {
  common: '通用设备',
  gaming: '游戏设备',
  transcend: '破限设备',
} as const satisfies Record<TemplateCategory, string>

const CATEGORY_ORDER: Record<TemplateCategory, number> = {
  common: 0,
  gaming: 1,
  transcend: 2,
}

interface TreeResponse {
  tree?: Array<{
    path?: string
    type?: string
    sha?: string
  }>
}

interface FileContentResponse {
  content?: string
  encoding?: string
  download_url?: string
}

export interface TemplateIndexLoadResult {
  source: OnlineTemplateSource
  items: OnlineTemplateIndexItem[]
  /**
   * 聚合文件 dist/templates.toml 的 blob sha（来自同一次 git tree 响应，免费拿到）。
   * 有它才值得尝试"一次拉全量"，同时也是整包详情的缓存版本号。
   */
  bundleSha?: string
}

/** 聚合文件路径：与 templates/** 同仓，结构是 `[templates."<文件名>"]`（见 config 仓的 bundle_templates.py）。 */
export const TEMPLATE_BUNDLE_PATH = 'dist/templates.toml'

export interface TemplateDetailLoadResult {
  id: string
  detail?: OnlineTemplateDetail
  error?: string
  version?: string
}

export interface LoadTemplateDetailsOptions {
  signal?: AbortSignal
  concurrency?: number
  chunkSize?: number
  onChunk?: (results: TemplateDetailLoadResult[]) => void
}

export class RateLimitError extends Error {
  constructor(message = 'API rate limit exceeded') {
    super(message)
    this.name = 'RateLimitError'
  }
}

export function isRateLimitError(error: unknown): boolean {
  return error instanceof Error && error.name === 'RateLimitError'
}

function getSourceConfig(source: OnlineTemplateSource) {
  return SOURCE_CONFIGS[source]
}

function buildContentsApiUrl(source: OnlineTemplateSource, path: string): string {
  const config = getSourceConfig(source)
  return `${config.apiBase}/repos/${config.owner}/${config.repo}/contents/${path}?ref=main`
}

function buildTreeUrl(source: OnlineTemplateSource): string {
  const config = getSourceConfig(source)
  return `${config.apiBase}/repos/${config.owner}/${config.repo}/git/trees/main?recursive=1`
}

function buildRawUrl(source: OnlineTemplateSource, path: string): string {
  return `${getSourceConfig(source).rawBase}/${path}`
}

function buildCdnUrl(source: OnlineTemplateSource, path: string): string {
  return `${getSourceConfig(source).cdnBase}/${path}`
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function isAbortError(error: unknown): boolean {
  return (
    (error instanceof DOMException && error.name === 'AbortError') ||
    (error instanceof Error && error.name === 'AbortError')
  )
}

function assertNotAborted(signal?: AbortSignal) {
  if (signal?.aborted) {
    throw new DOMException('The operation was aborted.', 'AbortError')
  }
}

/** 带中止响应的延时（重试间隔用）。 */
function delay(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const onAbort = () => {
      window.clearTimeout(timer)
      reject(new DOMException('The operation was aborted.', 'AbortError'))
    }
    const timer = window.setTimeout(() => {
      signal?.removeEventListener('abort', onAbort)
      resolve()
    }, ms)

    if (signal?.aborted) {
      onAbort()
      return
    }

    signal?.addEventListener('abort', onAbort, { once: true })
  })
}

function escapeShellArg(value: string): string {
  return value.replace(/'/g, "'\\''")
}

function decodeBase64Utf8(content: string): string {
  const binary = atob(content.replace(/\s+/g, ''))
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0))
  return new TextDecoder().decode(bytes)
}

function humanizeTemplateName(name: string): string {
  return name.replace(/[_-]+/g, ' ').trim()
}

function sortIndexItems(items: OnlineTemplateIndexItem[]): OnlineTemplateIndexItem[] {
  return [...items].sort((left, right) => {
    const categoryDiff = CATEGORY_ORDER[left.category] - CATEGORY_ORDER[right.category]
    if (categoryDiff !== 0) return categoryDiff

    const brandDiff = (left.brand || '').localeCompare(right.brand || '', undefined, {
      sensitivity: 'base',
    })
    if (brandDiff !== 0) return brandDiff

    return left.displayName.localeCompare(right.displayName, undefined, {
      sensitivity: 'base',
    })
  })
}

function buildTemplateIndexItem(
  source: OnlineTemplateSource,
  path: string,
  sha?: string
): OnlineTemplateIndexItem | null {
  const segments = path.split('/')
  if (segments.length < 3 || segments[0] !== 'templates') {
    return null
  }

  const category = segments[1] as TemplateCategory
  if (!(category in TEMPLATE_CATEGORIES)) {
    return null
  }

  const fileName = segments[segments.length - 1]
  if (!fileName.endsWith('.toml')) {
    return null
  }

  const name = fileName.replace(/\.toml$/i, '')
  const brand = segments.length > 3 ? segments[2] || null : null

  return {
    id: `${source}:${path}`,
    name,
    displayName: humanizeTemplateName(name),
    category,
    brand,
    path,
    sha,
    source,
    contentUrl: getDetailUrlCandidates(source, path)[0].url,
  }
}

async function requestTextViaFetch(
  url: string,
  signal?: AbortSignal,
  accept: string = FETCH_ACCEPT_JSON
): Promise<string> {
  assertNotAborted(signal)

  const controller = new AbortController()
  const onAbort = () => controller.abort()
  const timeoutId = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS)

  if (signal) {
    signal.addEventListener('abort', onAbort, { once: true })
  }

  try {
    const response = await fetch(url, {
      headers: {
        Accept: accept,
      },
      cache: 'no-store',
      signal: controller.signal,
    })

    if (!response.ok) {
      const remaining = response.headers.get('X-RateLimit-Remaining')
      const retryAfter = response.headers.get('Retry-After')
      if (response.status === 429 || remaining === '0' || retryAfter !== null) {
        throw new RateLimitError()
      }

      throw new Error(`HTTP ${response.status}`)
    }

    return await response.text()
  } finally {
    window.clearTimeout(timeoutId)
    if (signal) {
      signal.removeEventListener('abort', onAbort)
    }
  }
}

async function requestTextViaShell(
  url: string,
  accept: string = FETCH_ACCEPT_JSON
): Promise<string> {
  if (import.meta.env?.DEV) {
    throw new Error('Shell HTTP fallback is not available in development mode.')
  }

  const escapedUrl = escapeShellArg(url)
  const escapedAccept = escapeShellArg(accept)
  const timeout = SHELL_TIMEOUT_SECONDS

  // 不写死任何 root 方案的目录，运行时按「PATH → 通用 busybox 形态」自己找工具：
  //   · WebUI 的 exec 通常直接以 root 跑 `sh`（真机实测 id = uid=0，context u:r:magisk:s0），
  //     没有 su 参与，因此 PATH 就是 app 继承的标准 Android PATH —— 里面基本没有 curl/wget/busybox；
  //   · KernelSU 的 ksud su 会注入 /data/adb/ksu/bin，Magisk/WeaveMask 则不注入，
  //     所以只按 PATH 找不够，再补一层 `*/busybox`、`*/bin/busybox` 的通用 glob（不点名具体方案）。
  // 顺带：shell 没有 CORS 限制，所以 gitee raw（WebView fetch 拿不到，响应缺
  // Access-Control-Allow-Origin）也能借这条兜底直接抓。
  const script = [
    `CURL=$(command -v curl 2>/dev/null)`,
    `WGET=$(command -v wget 2>/dev/null)`,
    `BB=$(command -v busybox 2>/dev/null)`,
    `[ -z "$BB" ] && BB=$(ls /data/adb/*/busybox /data/adb/*/bin/busybox 2>/dev/null | head -n 1)`,
    // 不要用 exec 替换 shell：真机实测 libsu 抓不到 exec 之后的输出（返回空串），
    // 用 if/elif 顺序执行才能把 stdout 正常带回来。
    `if [ -n "$CURL" ]; then "$CURL" -fsSL --connect-timeout ${timeout} --max-time ${timeout} -H 'Accept: ${escapedAccept}' '${escapedUrl}'`,
    `elif [ -n "$WGET" ]; then "$WGET" -q -O - --timeout=${timeout} --header='Accept: ${escapedAccept}' '${escapedUrl}'`,
    `elif [ -n "$BB" ]; then "$BB" wget -q -O - -T ${timeout} --header='Accept: ${escapedAccept}' '${escapedUrl}'`,
    `else echo 'No usable HTTP tool (curl/wget/busybox) found' >&2; exit 127; fi`,
  ].join('; ')

  return await execCommand(script)
}

async function requestText(
  url: string,
  signal?: AbortSignal,
  accept: string = FETCH_ACCEPT_JSON
): Promise<string> {
  try {
    return await requestTextViaFetch(url, signal, accept)
  } catch (error) {
    if (isAbortError(error)) {
      throw error
    }

    if (isRateLimitError(error)) {
      throw error
    }

    const fallback = await requestTextViaShell(url, accept)
    if (!fallback.trim()) {
      throw error instanceof Error ? error : new Error('Empty HTTP response')
    }

    return fallback
  }
}

async function requestJson<T>(
  url: string,
  signal?: AbortSignal,
  accept: string = FETCH_ACCEPT_JSON
): Promise<T> {
  const text = await requestText(url, signal, accept)
  return JSON.parse(text) as T
}

function getDetailUrlCandidates(
  source: OnlineTemplateSource,
  path: string
): Array<{ url: string; accept: string }> {
  if (source === 'github') {
    return [
      { url: buildRawUrl('github', path), accept: FETCH_ACCEPT_TEXT },
      { url: buildCdnUrl('github', path), accept: FETCH_ACCEPT_TEXT },
    ]
  }

  // Gitee raw has no CORS headers (browser fetch is blocked) and the contents
  // API is rate-limited (60 requests/hour/IP), so prefer jsDelivr which serves
  // the GitHub mirror with CORS enabled and no API quota.
  return [
    { url: buildCdnUrl('gitee', path), accept: FETCH_ACCEPT_TEXT },
    { url: buildRawUrl('gitee', path), accept: FETCH_ACCEPT_TEXT },
    { url: buildContentsApiUrl('gitee', path), accept: FETCH_ACCEPT_JSON },
  ]
}

async function fetchTemplateContentViaContentsApi(
  source: OnlineTemplateSource,
  url: string,
  signal?: AbortSignal
): Promise<string> {
  const accept = source === 'github' ? FETCH_ACCEPT_GITHUB_JSON : FETCH_ACCEPT_JSON
  const response = await requestJson<unknown>(url, signal, accept)

  if (!isRecord(response)) {
    throw new Error('Template content response is invalid.')
  }

  const fileResponse = response as FileContentResponse
  if (typeof fileResponse.content === 'string' && fileResponse.encoding === 'base64') {
    return decodeBase64Utf8(fileResponse.content)
  }

  if (typeof fileResponse.download_url === 'string' && fileResponse.download_url) {
    return await requestText(fileResponse.download_url, signal, FETCH_ACCEPT_TEXT)
  }

  throw new Error('Template content is unavailable.')
}

async function loadTreeIndex(
  source: OnlineTemplateSource,
  signal?: AbortSignal
): Promise<TemplateIndexLoadResult> {
  const accept = source === 'github' ? FETCH_ACCEPT_GITHUB_JSON : FETCH_ACCEPT_JSON
  const response = await requestJson<TreeResponse>(buildTreeUrl(source), signal, accept)

  if (!Array.isArray(response.tree)) {
    throw new Error(`${source} tree response is invalid.`)
  }

  const items = response.tree
    .map((entry) => {
      if (entry.type !== 'blob' || typeof entry.path !== 'string') {
        return null
      }

      return buildTemplateIndexItem(source, entry.path, entry.sha)
    })
    .filter((item): item is OnlineTemplateIndexItem => item !== null)

  const bundleEntry = response.tree.find(
    (entry) => entry.type === 'blob' && entry.path === TEMPLATE_BUNDLE_PATH
  )

  return {
    source,
    items: sortIndexItems(items),
    bundleSha: bundleEntry?.sha,
  }
}

/**
 * 一次性拉取聚合文件，返回「文件名 → 详情」的映射。
 *
 * 聚合文件由 config 仓的 .github/scripts/bundle_templates.py 机械拼接生成：结构与单个模板文件一致
 * （只有 `[templates."<文件名>"]` 表），字段逐字保留，因此这里直接复用 sanitizeTemplate /
 * extractTemplateMeta，不需要任何新 schema。失败时抛错，由调用方退回逐文件路径。
 */
/**
 * 聚合文件的候选顺序（与逐文件详情不同）。
 *
 * 聚合文件整包只发 1 次请求，所以优先「浏览器能直接跨域取到」的路径：
 * gitee 源的 raw 没有 CORS（WebView fetch 必失败，只能靠 shell 兜底），而且
 * contents API 只有 60 次/小时/IP 的配额——因此顺序是 jsDelivr → contents API → gitee raw，
 * 既不在正常情况下浪费配额，也不在 jsDelivr 挂掉时把用户丢回 700+ 次逐文件请求。
 * （逐文件详情仍走 getDetailUrlCandidates：那里量大，必须优先无配额的 raw+shell。）
 */
function getBundleUrlCandidates(
  source: OnlineTemplateSource
): Array<{ url: string; accept: string }> {
  if (source === 'github') {
    return [
      { url: buildRawUrl('github', TEMPLATE_BUNDLE_PATH), accept: FETCH_ACCEPT_TEXT },
      { url: buildCdnUrl('github', TEMPLATE_BUNDLE_PATH), accept: FETCH_ACCEPT_TEXT },
    ]
  }

  return [
    { url: buildCdnUrl('gitee', TEMPLATE_BUNDLE_PATH), accept: FETCH_ACCEPT_TEXT },
    { url: buildContentsApiUrl('gitee', TEMPLATE_BUNDLE_PATH), accept: FETCH_ACCEPT_JSON },
    { url: buildRawUrl('gitee', TEMPLATE_BUNDLE_PATH), accept: FETCH_ACCEPT_TEXT },
  ]
}

export async function loadTemplateBundle(
  source: OnlineTemplateSource,
  signal?: AbortSignal
): Promise<Map<string, OnlineTemplateDetail>> {
  const candidates = getBundleUrlCandidates(source)
  let lastError: unknown = null

  // 聚合文件是整个在线库的命脉：失败就退回逐文件（700+ 次请求，真机实测 3.5 分钟量级），
  // 所以候选链整体重试一轮再放弃（CDN 冷缓存/瞬时抖动是最常见的失败原因）。
  for (let attempt = 0; attempt < TEMPLATE_BUNDLE_ATTEMPTS; attempt += 1) {
    if (attempt > 0) {
      await delay(TEMPLATE_BUNDLE_RETRY_DELAY_MS, signal)
      assertNotAborted(signal)
    }

    for (const candidate of candidates) {
      try {
        const content =
          candidate.accept === FETCH_ACCEPT_JSON
            ? await fetchTemplateContentViaContentsApi(source, candidate.url, signal)
            : await requestText(candidate.url, signal, FETCH_ACCEPT_TEXT)

        if (!content.trim()) {
          lastError = new Error('Template bundle is empty.')
          continue
        }

        const parsed = parseToml(content) as unknown
        if (!isRecord(parsed) || !isRecord(parsed.templates)) {
          lastError = new Error('Template bundle does not contain a valid templates section.')
          continue
        }

        const details = new Map<string, OnlineTemplateDetail>()
        for (const [key, value] of Object.entries(parsed.templates)) {
          if (!isRecord(value)) {
            continue
          }

          details.set(key, {
            template: sanitizeTemplate(value as Template),
            meta: extractTemplateMeta(value) as TemplateMeta | undefined,
          })
        }

        if (details.size === 0) {
          lastError = new Error('Template bundle contains no templates.')
          continue
        }

        return details
      } catch (error) {
        if (isAbortError(error)) {
          throw error
        }

        lastError = error
      }
    }
  }

  throw lastError instanceof Error ? lastError : new Error('Template bundle is unavailable.')
}

function getSourceFailoverOrder(preferredSource: OnlineTemplateSource): OnlineTemplateSource[] {
  return preferredSource === 'gitee' ? ['gitee', 'github'] : ['github', 'gitee']
}

export async function loadTemplateIndex(
  preferredSource: OnlineTemplateSource,
  signal?: AbortSignal
): Promise<TemplateIndexLoadResult> {
  const errors: string[] = []
  let rateLimited = false

  for (const source of getSourceFailoverOrder(preferredSource)) {
    try {
      return await loadTreeIndex(source, signal)
    } catch (error) {
      if (isAbortError(error)) {
        throw error
      }

      if (isRateLimitError(error)) {
        rateLimited = true
      }

      const message = error instanceof Error ? error.message : String(error)
      errors.push(`${source}: ${message}`)
    }
  }

  if (rateLimited) {
    throw new RateLimitError()
  }

  throw new Error(errors.join(' | ') || 'Failed to load template index.')
}

function parseTemplateDocument(content: string): OnlineTemplateDetail {
  const parsed = parseToml(content) as unknown

  if (!isRecord(parsed) || !isRecord(parsed.templates)) {
    throw new Error('Template TOML does not contain a valid templates section.')
  }

  const entries = Object.entries(parsed.templates)
  const firstTemplate = entries[0]?.[1]

  if (!firstTemplate) {
    throw new Error('Template TOML is empty.')
  }

  return {
    template: sanitizeTemplate(firstTemplate as Template),
    meta: extractTemplateMeta(firstTemplate) as TemplateMeta | undefined,
  }
}

async function fetchTemplateContent(
  item: OnlineTemplateIndexItem,
  signal?: AbortSignal
): Promise<string> {
  const candidates = getDetailUrlCandidates(item.source, item.path)
  let lastError: unknown = null

  for (const candidate of candidates) {
    try {
      if (candidate.accept === FETCH_ACCEPT_JSON) {
        return await fetchTemplateContentViaContentsApi(item.source, candidate.url, signal)
      }

      const content = await requestText(candidate.url, signal, FETCH_ACCEPT_TEXT)
      if (content.trim()) {
        return content
      }

      lastError = new Error(`Template content for "${item.path}" is empty.`)
    } catch (error) {
      if (isAbortError(error)) {
        throw error
      }

      lastError = error
    }
  }

  throw lastError instanceof Error
    ? lastError
    : new Error(`Template content for "${item.path}" is unavailable.`)
}

async function loadSingleTemplateDetail(
  item: OnlineTemplateIndexItem,
  signal?: AbortSignal
): Promise<TemplateDetailLoadResult> {
  try {
    const content = await fetchTemplateContent(item, signal)
    const detail = parseTemplateDocument(content)

    return {
      id: item.id,
      detail,
      version: item.sha,
    }
  } catch (error) {
    if (isAbortError(error)) {
      throw error
    }

    return {
      id: item.id,
      error: error instanceof Error ? error.message : String(error),
      version: item.sha,
    }
  }
}

export async function loadTemplateDetails(
  items: OnlineTemplateIndexItem[],
  options: LoadTemplateDetailsOptions = {}
): Promise<TemplateDetailLoadResult[]> {
  const queue = [...items]
  const results: TemplateDetailLoadResult[] = []
  const pendingChunk: TemplateDetailLoadResult[] = []
  const concurrency = Math.max(1, options.concurrency ?? DETAIL_CONCURRENCY)
  const chunkSize = Math.max(1, options.chunkSize ?? concurrency)

  const flushChunk = () => {
    if (pendingChunk.length === 0 || !options.onChunk) {
      return
    }

    options.onChunk([...pendingChunk])
    pendingChunk.length = 0
  }

  async function worker() {
    while (queue.length > 0) {
      assertNotAborted(options.signal)
      const nextItem = queue.shift()
      if (!nextItem) {
        return
      }

      const result = await loadSingleTemplateDetail(nextItem, options.signal)
      results.push(result)
      pendingChunk.push(result)

      if (pendingChunk.length >= chunkSize) {
        flushChunk()
      }
    }
  }

  await Promise.all(
    Array.from({ length: Math.min(concurrency, items.length || 1) }, () => {
      return worker()
    })
  )

  flushChunk()
  return results
}
