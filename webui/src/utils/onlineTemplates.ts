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
const INDEX_CONCURRENCY = 4
const DETAIL_CONCURRENCY = 6
const FETCH_ACCEPT_JSON = 'application/json'
const FETCH_ACCEPT_GITHUB_JSON = 'application/vnd.github+json'

const SOURCE_CONFIGS = {
  gitee: {
    owner: 'Seyud',
    repo: 'device_faker_config_mirror',
    apiBase: 'https://gitee.com/api/v5',
  },
  github: {
    owner: 'Seyud',
    repo: 'device_faker_config',
    apiBase: 'https://api.github.com',
  },
} as const satisfies Record<
  OnlineTemplateSource,
  {
    owner: string
    repo: string
    apiBase: string
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

interface DirectoryEntry {
  type: 'file' | 'dir'
  name: string
  path: string
  sha?: string
}

interface GithubTreeResponse {
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
}

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

function getSourceConfig(source: OnlineTemplateSource) {
  return SOURCE_CONFIGS[source]
}

function buildContentsApiUrl(source: OnlineTemplateSource, path: string): string {
  const config = getSourceConfig(source)
  return `${config.apiBase}/repos/${config.owner}/${config.repo}/contents/${path}?ref=main`
}

function buildGithubTreeUrl(): string {
  const config = getSourceConfig('github')
  return `${config.apiBase}/repos/${config.owner}/${config.repo}/git/trees/main?recursive=1`
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
    contentUrl: buildContentsApiUrl(source, path),
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
  const curlCommand = `curl -fsSL --connect-timeout ${SHELL_TIMEOUT_SECONDS} -H 'Accept: ${escapedAccept}' '${escapedUrl}'`
  const wgetCommand = `wget -q -O - --timeout=${SHELL_TIMEOUT_SECONDS} --header='Accept: ${escapedAccept}' '${escapedUrl}'`

  return await execCommand(`${curlCommand} || ${wgetCommand}`)
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

async function fetchDirectoryEntries(
  source: OnlineTemplateSource,
  path: string,
  signal?: AbortSignal
): Promise<DirectoryEntry[]> {
  const accept = source === 'github' ? FETCH_ACCEPT_GITHUB_JSON : FETCH_ACCEPT_JSON
  const response = await requestJson<unknown>(buildContentsApiUrl(source, path), signal, accept)

  if (!Array.isArray(response)) {
    throw new Error(`Directory listing for "${path}" is not an array.`)
  }

  return response
    .filter((entry): entry is Record<string, unknown> => isRecord(entry))
    .map(
      (entry): DirectoryEntry => ({
        type: entry.type === 'dir' ? 'dir' : 'file',
        name: typeof entry.name === 'string' ? entry.name : '',
        path: typeof entry.path === 'string' ? entry.path : '',
        sha: typeof entry.sha === 'string' ? entry.sha : undefined,
      })
    )
    .filter((entry) => Boolean(entry.name && entry.path))
}

async function walkGiteeCategoryIndex(
  category: TemplateCategory,
  signal?: AbortSignal
): Promise<OnlineTemplateIndexItem[]> {
  const queue = [`templates/${category}`]
  const items: OnlineTemplateIndexItem[] = []

  async function worker() {
    while (queue.length > 0) {
      assertNotAborted(signal)
      const currentPath = queue.shift()
      if (!currentPath) {
        return
      }

      const entries = await fetchDirectoryEntries('gitee', currentPath, signal)
      for (const entry of entries) {
        if (entry.type === 'dir') {
          queue.push(entry.path)
          continue
        }

        const item = buildTemplateIndexItem('gitee', entry.path, entry.sha)
        if (item) {
          items.push(item)
        }
      }
    }
  }

  await Promise.all(
    Array.from({ length: INDEX_CONCURRENCY }, () => {
      return worker()
    })
  )

  return items
}

async function loadGiteeIndex(signal?: AbortSignal): Promise<TemplateIndexLoadResult> {
  const categories = Object.keys(TEMPLATE_CATEGORIES) as TemplateCategory[]
  const results = await Promise.all(
    categories.map((category) => walkGiteeCategoryIndex(category, signal))
  )

  return {
    source: 'gitee',
    items: sortIndexItems(results.flat()),
  }
}

async function loadGithubIndex(signal?: AbortSignal): Promise<TemplateIndexLoadResult> {
  const response = await requestJson<GithubTreeResponse>(
    buildGithubTreeUrl(),
    signal,
    FETCH_ACCEPT_GITHUB_JSON
  )

  if (!Array.isArray(response.tree)) {
    throw new Error('GitHub tree response is invalid.')
  }

  const items = response.tree
    .map((entry) => {
      if (entry.type !== 'blob' || typeof entry.path !== 'string') {
        return null
      }

      return buildTemplateIndexItem('github', entry.path, entry.sha)
    })
    .filter((item): item is OnlineTemplateIndexItem => item !== null)

  return {
    source: 'github',
    items: sortIndexItems(items),
  }
}

function getSourceFailoverOrder(preferredSource: OnlineTemplateSource): OnlineTemplateSource[] {
  return preferredSource === 'gitee' ? ['gitee', 'github'] : ['github', 'gitee']
}

export async function loadTemplateIndex(
  preferredSource: OnlineTemplateSource,
  signal?: AbortSignal
): Promise<TemplateIndexLoadResult> {
  const errors: string[] = []

  for (const source of getSourceFailoverOrder(preferredSource)) {
    try {
      if (source === 'gitee') {
        return await loadGiteeIndex(signal)
      }

      return await loadGithubIndex(signal)
    } catch (error) {
      if (isAbortError(error)) {
        throw error
      }

      const message = error instanceof Error ? error.message : String(error)
      errors.push(`${source}: ${message}`)
    }
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
  const accept = item.source === 'github' ? FETCH_ACCEPT_GITHUB_JSON : FETCH_ACCEPT_JSON
  const response = await requestJson<unknown>(item.contentUrl, signal, accept)

  if (!isRecord(response)) {
    throw new Error(`Template content for "${item.path}" is invalid.`)
  }

  const fileResponse = response as FileContentResponse
  if (typeof fileResponse.content === 'string' && fileResponse.encoding === 'base64') {
    return decodeBase64Utf8(fileResponse.content)
  }

  if (typeof fileResponse.download_url === 'string' && fileResponse.download_url) {
    return await requestText(fileResponse.download_url, signal, 'text/plain')
  }

  throw new Error(`Template content for "${item.path}" is unavailable.`)
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
