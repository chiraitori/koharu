#!/usr/bin/env bun

import { mkdir, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'

// Minimal vanilla Protobuf wire parser (Zero external dependencies)
type ProtoField = {
  fieldNo: number
  wireType: number
  val?: bigint
  data?: Buffer
}

function decodeProto(buffer: Buffer): ProtoField[] {
  let pos = 0
  const fields: ProtoField[] = []
  while (pos < buffer.length) {
    let byte = buffer[pos++]
    let tag = byte & 0x7f
    let shift = 7
    while (byte >= 0x80) {
      byte = buffer[pos++]
      tag |= (byte & 0x7f) << shift
      shift += 7
    }
    const fieldNo = tag >> 3
    const wireType = tag & 0x07

    if (wireType === 0) {
      // Varint
      let val = 0n
      let s = 0n
      let b: bigint
      do {
        b = BigInt(buffer[pos++])
        val |= (b & 0x7fn) << s
        s += 7n
      } while (b >= 0x80n)
      fields.push({ fieldNo, wireType, val })
    } else if (wireType === 2) {
      // Length-delimited (string / bytes / nested message)
      let len = 0
      let s = 0
      let b: number
      do {
        b = buffer[pos++];
        len |= (b & 0x7f) << s
        s += 7
      } while (b >= 0x80)
      const data = buffer.subarray(pos, pos + len)
      pos += len
      fields.push({ fieldNo, wireType, data })
    } else if (wireType === 1) {
      // 64-bit
      pos += 8
    } else if (wireType === 5) {
      // 32-bit
      pos += 4
    } else {
      break
    }
  }
  return fields
}

interface ChapterMeta {
  chapterId: number
  chapterName: string
  description: string
  publishedDate?: string
  badge?: number
}

interface ViewerData {
  titleName: string
  titleId: number
  aesKey: string
  aesIv: string
  currentChapter: ChapterMeta
  pages: {
    index: number
    url: string
    isEncrypted: boolean
  }[]
  chapterList: ChapterMeta[]
}

export async function fetchViewerV2(titleId: number, chapterId: number): Promise<ViewerData> {
  const url = `https://manga-one.com/api/client?rq=viewer_v2&title_id=${titleId}&chapter_id=${chapterId}&page=1&limit=10&sort_type=desc&list_type=chapter&free_point=0&event_point=0&paid_point=0`
  const res = await fetch(url, { method: 'POST' })
  if (!res.ok) {
    throw new Error(`Failed to fetch viewer data: HTTP ${res.status}`)
  }

  const rawBuf = Buffer.from(await res.arrayBuffer())
  const topFields = decodeProto(rawBuf)

  // Field 3 = aesKey, Field 4 = aesIv
  const aesKey = topFields.find(f => f.fieldNo === 3)?.data?.toString('utf8') || ''
  const aesIv = topFields.find(f => f.fieldNo === 4)?.data?.toString('utf8') || ''

  // Field 5 = currentTitle
  let titleName = `Title_${titleId}`
  const currentTitleField = topFields.find(f => f.fieldNo === 5)
  if (currentTitleField?.data) {
    const ctSub = decodeProto(currentTitleField.data)
    const titleSub = ctSub.find(f => f.fieldNo === 1)
    if (titleSub?.data) {
      const tFields = decodeProto(titleSub.data)
      const tName = tFields.find(f => f.fieldNo === 2)?.data?.toString('utf8')
      if (tName) titleName = tName
    }
  }

  // Field 7 = currentChapter
  let currentChapter: ChapterMeta = { chapterId, chapterName: `Chapter_${chapterId}`, description: '' }
  const curChField = topFields.find(f => f.fieldNo === 7)
  if (curChField?.data) {
    const chSub = decodeProto(curChField.data)
    const id = Number(chSub.find(f => f.fieldNo === 1)?.val ?? chapterId)
    const name = chSub.find(f => f.fieldNo === 2)?.data?.toString('utf8') ?? `Chapter_${id}`
    const desc = chSub.find(f => f.fieldNo === 3)?.data?.toString('utf8') ?? ''
    currentChapter = { chapterId: id, chapterName: name, description: desc }
  }

  // Field 1 = pages (repeated)
  const pageFields = topFields.filter(f => f.fieldNo === 1)
  const pages: ViewerData['pages'] = []
  let pageIdx = 1

  for (const pf of pageFields) {
    if (!pf.data) continue
    const pSub = decodeProto(pf.data)
    // Inside page: field 1 (image) or field 2 (imageWithAction)
    const imgField = pSub.find(f => f.fieldNo === 1 || f.fieldNo === 2)
    if (imgField?.data) {
      const imgSub = decodeProto(imgField.data)
      const imgUrl = imgSub.find(f => f.fieldNo === 1)?.data?.toString('utf8')
      if (imgUrl) {
        pages.push({
          index: pageIdx++,
          url: imgUrl,
          isEncrypted: imgUrl.includes('.enc'),
        })
      }
    }
  }

  // Field 11 = chapterList
  const chapterList: ChapterMeta[] = []
  const chListField = topFields.find(f => f.fieldNo === 11)
  if (chListField?.data) {
    const clSub = decodeProto(chListField.data)
    const items = clSub.filter(f => f.fieldNo === 1)
    for (const item of items) {
      if (!item.data) continue
      const itemSub = decodeProto(item.data)
      const id = Number(itemSub.find(f => f.fieldNo === 1)?.val ?? 0)
      const name = itemSub.find(f => f.fieldNo === 2)?.data?.toString('utf8') ?? `Chapter_${id}`
      const desc = itemSub.find(f => f.fieldNo === 3)?.data?.toString('utf8') ?? ''
      const date = itemSub.find(f => f.fieldNo === 5)?.data?.toString('utf8')
      const badge = Number(itemSub.find(f => f.fieldNo === 7)?.val ?? 0)
      chapterList.push({ chapterId: id, chapterName: name, description: desc, publishedDate: date, badge })
    }
  }

  return {
    titleName,
    titleId,
    aesKey,
    aesIv,
    currentChapter,
    pages,
    chapterList,
  }
}

function sanitizeName(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, '_').trim()
}

async function downloadAndDecryptPage(
  url: string,
  outPath: string,
  isEncrypted: boolean,
  keyBuf: Buffer,
  ivBuf: Buffer,
): Promise<void> {
  const res = await fetch(url)
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} fetching ${url}`)
  }

  const rawBytes = Buffer.from(await res.arrayBuffer())
  let finalBytes: Buffer

  if (isEncrypted) {
    const decipher = crypto.createDecipheriv('aes-256-cbc', keyBuf, ivBuf)
    finalBytes = Buffer.concat([decipher.update(rawBytes), decipher.final()])
  } else {
    finalBytes = rawBytes
  }

  // Verify WebP header
  const riff = finalBytes.subarray(0, 4).toString('ascii')
  const webp = finalBytes.subarray(8, 12).toString('ascii')
  if (riff !== 'RIFF' || webp !== 'WEBP') {
    throw new Error(`Decrypted payload is not valid WebP (header: ${riff}...${webp})`)
  }

  await writeFile(outPath, finalBytes)
}

async function main() {
  const args = process.argv.slice(2)
  const downloadAll = args.includes('--all')
  const cleanArgs = args.filter(a => !a.startsWith('--'))
  const target = cleanArgs[0] || 'https://manga-one.com/manga/28342/chapter/356962'

  let titleId = 0
  let chapterId = 0

  const urlMatch = target.match(/\/manga\/(\d+)\/chapter\/(\d+)/)
  const titleOnlyMatch = target.match(/\/manga\/(\d+)/)

  if (urlMatch) {
    titleId = parseInt(urlMatch[1], 10)
    chapterId = parseInt(urlMatch[2], 10)
  } else if (titleOnlyMatch) {
    titleId = parseInt(titleOnlyMatch[1], 10)
  } else if (/^\d+$/.test(target)) {
    titleId = parseInt(target, 10)
  } else {
    console.error('Usage: bun scripts/download_mangaone.ts <url-or-titleId> [--all]')
    process.exit(1)
  }

  console.info(`[MangaOne Fetcher] Initializing for Title ${titleId}...`)

  if (!chapterId) {
    chapterId = 356710
  }

  const initialViewer = await fetchViewerV2(titleId, chapterId)
  const titleName = initialViewer.titleName
  console.info(`[MangaOne Fetcher] Title: "${titleName}" (ID: ${titleId})`)

  const outBase = path.resolve('data/manga', `${titleId}_${sanitizeName(titleName)}`)
  await mkdir(outBase, { recursive: true })

  const chaptersToDownload: ChapterMeta[] = []
  if (urlMatch && !downloadAll) {
    console.info(`[MangaOne Fetcher] Mode: Single Chapter (ID: ${chapterId})`)
    const targetCh = initialViewer.chapterList.find(c => c.chapterId === chapterId) || initialViewer.currentChapter
    chaptersToDownload.push(targetCh)
  } else if (initialViewer.chapterList.length > 0) {
    console.info(`[MangaOne Fetcher] Discovered ${initialViewer.chapterList.length} total chapters in series:`)
    for (const ch of initialViewer.chapterList) {
      console.info(`  - ${ch.chapterName}: ${ch.description} (Badge: ${ch.badge})`)
      chaptersToDownload.push(ch)
    }
  } else {
    chaptersToDownload.push(initialViewer.currentChapter)
  }


  for (const ch of chaptersToDownload) {
    console.info(`\n--> Checking ${ch.chapterName} (${ch.description})...`)
    let viewer: ViewerData
    if (ch.chapterId === initialViewer.currentChapter.chapterId) {
      viewer = initialViewer
    } else {
      try {
        viewer = await fetchViewerV2(titleId, ch.chapterId)
      } catch (err: any) {
        console.warn(`    Failed to fetch chapter ${ch.chapterId}: ${err.message}`)
        continue
      }
    }

    if (viewer.pages.length === 0) {
      console.info(`    Chapter ${ch.chapterName} has 0 pages available (requires points/auth), skipping.`)
      continue
    }

    const chFolder = path.join(outBase, `${sanitizeName(ch.chapterName)}_${sanitizeName(ch.description)}`)
    await mkdir(chFolder, { recursive: true })

    const keyBuf = Buffer.from(viewer.aesKey, 'hex')
    const ivBuf = Buffer.from(viewer.aesIv, 'hex')

    console.info(`    Downloading ${viewer.pages.length} pages to ${chFolder}...`)

    for (const page of viewer.pages) {
      const pageNumStr = String(page.index).padStart(2, '0')
      const targetPath = path.join(chFolder, `${pageNumStr}.webp`)

      if (existsSync(targetPath)) {
        continue
      }

      await downloadAndDecryptPage(page.url, targetPath, page.isEncrypted, keyBuf, ivBuf)
      process.stdout.write(`\r    [${pageNumStr}/${viewer.pages.length}] Decrypted and saved.`)
    }
    console.info(`\n    ✓ Completed ${ch.chapterName}!`)
  }

  console.info(`\n[MangaOne Fetcher] All available chapters downloaded successfully to:\n${outBase}`)
}

main().catch(err => {
  console.error('[MangaOne Fetcher] Error:', err)
  process.exit(1)
})
