import { parse as parseToml } from 'smol-toml'
import type { Template } from '../types'
import { execCommand } from './ksu'

const GITEE_OWNER = 'Seyud'
const GITEE_REPO = 'device_faker_config_mirror'
const GITEE_API_BASE = 'https://gitee.com/api/v5'
const GITEE_RAW_BASE = `https://gitee.com/${GITEE_OWNER}/${GITEE_REPO}/raw/main`

export const TEMPLATE_CATEGORIES = {
  common: '通用设备',
  gaming: '游戏设备',
  transcend: '破限设备',
} as const

export type TemplateCategory = keyof typeof TEMPLATE_CATEGORIES

export interface OnlineTemplate {
  name: string
  displayName: string
  category: TemplateCategory
  brand: string | null
  path: string
  downloadUrl: string
  template?: Template
}

export interface OnlineTemplatesResult {
  templates: OnlineTemplate[]
  brands: string[]
}

const brandCache = new Map<TemplateCategory, string[]>()

async function fetchDirsFromAPI(path: string): Promise<string[]> {
  const url = `${GITEE_API_BASE}/repos/${GITEE_OWNER}/${GITEE_REPO}/contents/${path}?ref=main`
  try {
    const response = await fetch(url)
    if (!response.ok) return []
    const jsonStr = await response.text()
    const files = JSON.parse(jsonStr)
    if (!Array.isArray(files)) return []
    return files
      .filter((f: { type: string }) => f.type === 'dir')
      .map((f: { name: string }) => f.name)
  } catch {
    return []
  }
}

async function getBrandCategories(category: TemplateCategory): Promise<string[]> {
  if (brandCache.has(category)) {
    return brandCache.get(category)!
  }

  const path = `templates/${category}`
  const dirs = await fetchDirsFromAPI(path)
  const brands = dirs.filter((dir) => !dir.startsWith('.'))

  brandCache.set(category, brands)
  return brands
}

export async function fetchBrandsByCategory(category: TemplateCategory): Promise<string[]> {
  return getBrandCategories(category)
}

export async function fetchAllBrands(): Promise<string[]> {
  const categories = Object.keys(TEMPLATE_CATEGORIES) as TemplateCategory[]
  const allBrands = new Set<string>()

  await Promise.all(
    categories.map(async (cat) => {
      const brands = await getBrandCategories(cat)
      brands.forEach((b) => allBrands.add(b))
    })
  )

  return Array.from(allBrands).sort()
}

async function getTomlFilesFromHTML(
  path: string,
  category: TemplateCategory,
  brands: string[]
): Promise<OnlineTemplate[]> {
  const templates: OnlineTemplate[] = []
  const url = `https://gitee.com/${GITEE_OWNER}/${GITEE_REPO}/tree/main/${path}`

  try {
    const response = await fetch(url)
    if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`)
    const html = await response.text()

    const escapedPath = path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const fileRegex = new RegExp(
      `/${GITEE_OWNER}/${GITEE_REPO}/blob/main/(${escapedPath}/[^"]+\\.toml)`,
      'g'
    )
    const dirRegex = new RegExp(
      `/${GITEE_OWNER}/${GITEE_REPO}/tree/main/(${escapedPath}/[^"/]+)(?=")`,
      'g'
    )

    let match
    const foundFiles = new Set<string>()
    while ((match = fileRegex.exec(html)) !== null) {
      foundFiles.add(match[1])
    }

    const foundDirs = new Set<string>()
    const dirMatches = html.matchAll(dirRegex)
    for (const dirMatch of dirMatches) {
      const dirPath = dirMatch[1]
      if (!foundDirs.has(dirPath) && dirPath !== path) {
        foundDirs.add(dirPath)
      }
    }

    for (const filePath of foundFiles) {
      const fileName = filePath.split('/').pop()!.replace('.toml', '')
      templates.push({
        name: fileName,
        displayName: fileName.replace(/_/g, ' '),
        category,
        brand: null,
        path: filePath,
        downloadUrl: `${GITEE_RAW_BASE}/${filePath}`,
      })
    }

    const brandDirs: { dirPath: string; brand: string }[] = []
    const normalDirs: string[] = []

    for (const dirPath of foundDirs) {
      const dirName = dirPath.split('/').pop()!
      if (brands.includes(dirName)) {
        brandDirs.push({ dirPath, brand: dirName })
      } else {
        normalDirs.push(dirPath)
      }
    }

    for (const { dirPath, brand } of brandDirs) {
      const subTemplates = await getTomlFilesFromHTML(dirPath, category, brands)
      subTemplates.forEach((t) => {
        t.brand = brand
      })
      templates.push(...subTemplates)
    }

    for (const dirPath of normalDirs) {
      const subTemplates = await getTomlFilesFromHTML(dirPath, category, brands)
      templates.push(...subTemplates)
    }

    return templates
  } catch (error) {
    console.error(`Failed to parse HTML from ${path}:`, error)
    throw error
  }
}

async function getTomlFilesFromAPI(
  path: string,
  category: TemplateCategory,
  brands: string[]
): Promise<OnlineTemplate[]> {
  const templates: OnlineTemplate[] = []
  const url = `${GITEE_API_BASE}/repos/${GITEE_OWNER}/${GITEE_REPO}/contents/${path}?ref=main`

  try {
    const response = await fetch(url)
    if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`)
    const jsonStr = await response.text()
    const files = JSON.parse(jsonStr)

    if (!Array.isArray(files)) throw new Error('API response is not an array')

    for (const file of files) {
      if (file.type === 'file' && file.name.endsWith('.toml')) {
        templates.push({
          name: file.name.replace('.toml', ''),
          displayName: file.name.replace('.toml', '').replace(/_/g, ' '),
          category,
          brand: null,
          path: file.path,
          downloadUrl: `${GITEE_RAW_BASE}/${file.path}`,
        })
      } else if (file.type === 'dir') {
        const dirName = file.name
        if (brands.includes(dirName)) {
          const brandTemplates = await getTomlFilesFromAPI(file.path, category, brands)
          brandTemplates.forEach((t) => {
            t.brand = dirName
          })
          templates.push(...brandTemplates)
        } else {
          const subTemplates = await getTomlFilesFromAPI(file.path, category, brands)
          templates.push(...subTemplates)
        }
      }
    }

    return templates
  } catch (error) {
    console.error(`Failed to fetch from API ${path}:`, error)
    throw error
  }
}

async function getTomlFiles(category: TemplateCategory): Promise<OnlineTemplate[]> {
  const path = `templates/${category}`
  const brands = await getBrandCategories(category)

  try {
    const templates = await getTomlFilesFromAPI(path, category, brands)
    if (templates.length > 0) return templates
  } catch (apiError) {
    console.warn(`API method failed for ${category}:`, apiError)
  }

  try {
    const templates = await getTomlFilesFromHTML(path, category, brands)
    if (templates.length > 0) return templates
  } catch (htmlError) {
    console.error(`HTML method failed for ${category}:`, htmlError)
  }

  return []
}

export async function fetchOnlineTemplates(): Promise<OnlineTemplatesResult> {
  const categories = Object.keys(TEMPLATE_CATEGORIES) as TemplateCategory[]
  const results = await Promise.all(categories.map((cat) => getTomlFiles(cat)))
  const templates = results.flat()
  const brands = await fetchAllBrands()

  return { templates, brands }
}

export async function downloadTemplate(onlineTemplate: OnlineTemplate): Promise<Template | null> {
  const CLI_PATH = '/data/adb/modules/device_faker/bin/device_faker_cli'
  const tempFile = `/data/local/tmp/template_${Date.now()}.toml`

  try {
    await execCommand(`${CLI_PATH} import -s "${onlineTemplate.downloadUrl}" -o "${tempFile}"`)
    const content = await execCommand(`cat "${tempFile}"`)
    await execCommand(`rm -f "${tempFile}"`).catch(() => {})

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const parsed = parseToml(content) as any

    if (parsed.templates) {
      const templateKey = Object.keys(parsed.templates)[0]
      if (templateKey) {
        return parsed.templates[templateKey] as Template
      }
    }

    return null
  } catch (error) {
    console.error(`Failed to download template ${onlineTemplate.name}:`, error)
    return null
  }
}

export async function downloadTemplates(
  onlineTemplates: OnlineTemplate[]
): Promise<OnlineTemplate[]> {
  const results = await Promise.all(
    onlineTemplates.map(async (t) => {
      const template = await downloadTemplate(t)
      return { ...t, template: template || undefined }
    })
  )
  return results.filter((t) => t.template !== undefined)
}
