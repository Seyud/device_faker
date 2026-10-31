/**
 * 最小 ZIP 读取器：只为「从 Magisk 模块 ZIP 里取出 system.prop」这一件事服务。
 *
 * 不依赖设备、不落盘——ZIP 在 WebView 里直接解析，因此不需要把用户的 zip
 * 上传到 /data/local/tmp 再 fork `device_faker_cli`。
 *
 * 支持 stored(0) 与 deflate(8) 两种存储方式（模块 zip 只会用这两种）；
 * ZIP64 直接报错（模块 zip 远不会超过 4 GiB）。
 */

const SIG_EOCD = 0x06054b50
const SIG_CENTRAL_DIR = 0x02014b50
const SIG_LOCAL_FILE = 0x04034b50
const EOCD_MIN_SIZE = 22
const MAX_COMMENT = 0xffff

const METHOD_STORED = 0
const METHOD_DEFLATE = 8

export interface ZipEntry {
  /** 归档内路径（正斜杠分隔），目录以 `/` 结尾 */
  name: string
  /** 解压后的原始字节 */
  data: Uint8Array
}

function findEndOfCentralDirectory(view: DataView): number {
  const minOffset = Math.max(0, view.byteLength - EOCD_MIN_SIZE - MAX_COMMENT)
  for (let offset = view.byteLength - EOCD_MIN_SIZE; offset >= minOffset; offset -= 1) {
    if (view.getUint32(offset, true) === SIG_EOCD) {
      return offset
    }
  }
  throw new Error('not a ZIP archive (missing end of central directory)')
}

async function inflateRaw(data: Uint8Array): Promise<Uint8Array> {
  if (typeof DecompressionStream === 'undefined') {
    throw new Error('this WebView cannot inflate deflate entries (no DecompressionStream)')
  }

  const stream = new Blob([data as unknown as BlobPart])
    .stream()
    .pipeThrough(new DecompressionStream('deflate-raw'))
  return new Uint8Array(await new Response(stream).arrayBuffer())
}

/**
 * 读取 ZIP 全部条目（按中央目录顺序）。目录条目会被跳过。
 */
export async function readZipEntries(buffer: ArrayBuffer): Promise<ZipEntry[]> {
  const view = new DataView(buffer)
  const bytes = new Uint8Array(buffer)
  const eocd = findEndOfCentralDirectory(view)

  const totalEntries = view.getUint16(eocd + 10, true)
  const centralDirSize = view.getUint32(eocd + 12, true)
  const centralDirOffset = view.getUint32(eocd + 16, true)

  if (totalEntries === 0xffff || centralDirSize === 0xffffffff || centralDirOffset === 0xffffffff) {
    throw new Error('ZIP64 archives are not supported')
  }

  const decoder = new TextDecoder()
  const entries: ZipEntry[] = []
  let cursor = centralDirOffset

  for (let index = 0; index < totalEntries; index += 1) {
    if (view.getUint32(cursor, true) !== SIG_CENTRAL_DIR) {
      throw new Error('corrupted ZIP (bad central directory signature)')
    }

    const method = view.getUint16(cursor + 10, true)
    const compressedSize = view.getUint32(cursor + 20, true)
    const nameLength = view.getUint16(cursor + 28, true)
    const extraLength = view.getUint16(cursor + 30, true)
    const commentLength = view.getUint16(cursor + 32, true)
    const localOffset = view.getUint32(cursor + 42, true)
    const name = decoder.decode(bytes.subarray(cursor + 46, cursor + 46 + nameLength))

    cursor += 46 + nameLength + extraLength + commentLength

    if (name.endsWith('/')) {
      continue
    }

    if (view.getUint32(localOffset, true) !== SIG_LOCAL_FILE) {
      throw new Error(`corrupted ZIP (bad local header for ${name})`)
    }

    // 本地头里的 name/extra 长度可能与中央目录不同，必须按本地头重新算数据偏移。
    const localNameLength = view.getUint16(localOffset + 26, true)
    const localExtraLength = view.getUint16(localOffset + 28, true)
    const dataOffset = localOffset + 30 + localNameLength + localExtraLength
    const raw = bytes.subarray(dataOffset, dataOffset + compressedSize)

    let data: Uint8Array
    if (method === METHOD_STORED) {
      data = raw
    } else if (method === METHOD_DEFLATE) {
      data = await inflateRaw(raw)
    } else {
      throw new Error(`unsupported compression method ${method} in ${name}`)
    }

    entries.push({ name, data })
  }

  return entries
}

/**
 * 按「先根后子、同级按归档顺序」的深度优先顺序找第一个匹配条目，
 * 与 device_faker_cli 的「解压到临时目录后 WalkDir 遍历」顺序一致。
 */
