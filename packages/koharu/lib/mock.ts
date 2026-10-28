'use client'

import { commands } from '@koharu/bridge/protocol'
import type {
  CanvasState,
  FontFamily,
  Model,
  Page,
  PageSummary,
  Preferences,
  ProjectInfo,
} from '@koharu/bridge/protocol'

export const isBrowserMock =
  typeof window !== 'undefined' &&
  !Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)

export const MOCK_PROJECT: ProjectInfo = {
  name: 'Momiji to Kouyou',
  revision: 31,
  active_page: 'page-31',
  can_undo: true,
  can_redo: false,
}

export const MOCK_PAGES: PageSummary[] = Array.from({ length: 32 }, (_, i) => {
  const num = String(i + 1).padStart(2, '0')
  return {
    id: `page-${num}`,
    label: `${num}.png`,
    size: { width: 723, height: 1024 },
    source_asset: `/showcase/${num}.png`,
    layer_count: i === 30 ? 6 : (i % 5) + 4,
  }
})

export const MOCK_PAGE: Page = {
  id: 'page-31',
  label: '31.png',
  size: { width: 723, height: 1024 },
  layers: [
    {
      type: 'text',
      id: 'layer-1',
      parent: null,
      geometry: {
        points: [
          { x: 520, y: 170 },
          { x: 675, y: 170 },
          { x: 675, y: 335 },
          { x: 520, y: 335 },
        ],
      },
      angle_degrees: 0,
      visibility: { visible: true, opacity: 1 },
      content: {
        id: 'content-1',
        source: { text: 'ダメ…かな…？', language: 'ja' },
        translation: { text: 'IS IT... NOT OKAY...?', language: 'en' },
        role: 'dialogue',
        source_region: null,
      },
      typography: {
        preferred_font: 'CCWildWords',
        font_weight: 400,
        font_style: 'normal',
        size: 16,
        auto_fit: true,
        color: [0, 0, 0, 255],
        stroke_color: null,
        stroke_width: null,
        alignment: 'Center',
        writing_mode: 'Horizontal',
      },
      layout: 'paragraph',
      automatic_region: null,
    },
    {
      type: 'text',
      id: 'layer-2',
      parent: null,
      geometry: {
        points: [
          { x: 35, y: 220 },
          { x: 190, y: 220 },
          { x: 190, y: 395 },
          { x: 35, y: 395 },
        ],
      },
      angle_degrees: 0,
      visibility: { visible: true, opacity: 1 },
      content: {
        id: 'content-2',
        source: { text: '間違いだらけの二人の関係。', language: 'ja' },
        translation: { text: 'A RELATIONSHIP BETWEEN TWO PEOPLE FULL OF MISTAKES.', language: 'en' },
        role: 'dialogue',
        source_region: null,
      },
      typography: {
        preferred_font: 'CCWildWords',
        font_weight: 400,
        font_style: 'normal',
        size: 14,
        auto_fit: true,
        color: [0, 0, 0, 255],
        stroke_color: null,
        stroke_width: null,
        alignment: 'Center',
        writing_mode: 'Horizontal',
      },
      layout: 'paragraph',
      automatic_region: null,
    },
    {
      type: 'text',
      id: 'layer-3',
      parent: null,
      geometry: {
        points: [
          { x: 40, y: 725 },
          { x: 315, y: 725 },
          { x: 315, y: 815 },
          { x: 40, y: 815 },
        ],
      },
      angle_degrees: 0,
      visibility: { visible: true, opacity: 1 },
      content: {
        id: 'content-3',
        source: { text: '２巻に続く、ドキドキのデートが始まる！！！', language: 'ja' },
        translation: { text: 'TO BE CONTINUED IN VOL. 2: THE HEART-POUNDING DATE BEGINS!!!', language: 'en' },
        role: 'dialogue',
        source_region: null,
      },
      typography: {
        preferred_font: 'CCWildWords',
        font_weight: 700,
        font_style: 'normal',
        size: 15,
        auto_fit: true,
        color: [0, 0, 0, 255],
        stroke_color: null,
        stroke_width: null,
        alignment: 'Center',
        writing_mode: 'Horizontal',
      },
      layout: 'paragraph',
      automatic_region: null,
    },
    {
      type: 'text',
      id: 'layer-4',
      parent: null,
      geometry: {
        points: [
          { x: 25, y: 25 },
          { x: 245, y: 25 },
          { x: 245, y: 60 },
          { x: 25, y: 60 },
        ],
      },
      angle_degrees: 0,
      visibility: { visible: true, opacity: 1 },
      content: {
        id: 'content-4',
        source: { text: '紅葉と黄葉', language: 'ja' },
        translation: { text: 'MOMIJI TO KOUYOU', language: 'en' },
        role: 'dialogue',
        source_region: null,
      },
      typography: {
        preferred_font: 'CCWildWords',
        font_weight: 700,
        font_style: 'normal',
        size: 14,
        auto_fit: true,
        color: [0, 0, 0, 255],
        stroke_color: null,
        stroke_width: null,
        alignment: 'Start',
        writing_mode: 'Horizontal',
      },
      layout: 'paragraph',
      automatic_region: null,
    },
    {
      type: 'raster',
      id: 'layer-cleanup',
      parent: null,
      name: 'Inpainting / Cleanup',
      kind: 'cleanup',
      image: null,
      visibility: { visible: true, opacity: 1 },
    },
    {
      type: 'artwork',
      id: 'layer-artwork',
      parent: null,
      geometry: {
        points: [
          { x: 0, y: 0 },
          { x: 723, y: 0 },
          { x: 723, y: 1024 },
          { x: 0, y: 1024 },
        ],
      },
      image: '/showcase/31.png',
      visibility: { visible: true, opacity: 1 },
    },
  ],
  regions: [],
}

export const MOCK_CANVAS_STATE: CanvasState = {
  page: 'page-31',
  revision: 31,
  generation: 1,
  size: [723, 1024],
  element_frames: [
    {
      element: 'layer-1',
      frame: { x: 520, y: 170, width: 155, height: 165, angle_degrees: 0 },
    },
    {
      element: 'layer-2',
      frame: { x: 35, y: 220, width: 155, height: 175, angle_degrees: 0 },
    },
    {
      element: 'layer-3',
      frame: { x: 40, y: 725, width: 275, height: 90, angle_degrees: 0 },
    },
    {
      element: 'layer-4',
      frame: { x: 25, y: 25, width: 220, height: 35, angle_degrees: 0 },
    },
  ],
}

export const MOCK_MODELS: Model[] = [
  {
    provider: 'local',
    model: 'qwen3.8-27b-uncensored',
    name: 'Qwen 3.8 27B Uncensored',
    quantizations: [],
    vision: true,
    reasoning: true,
  },
  {
    provider: 'gemini',
    model: 'gemini-2.5-flash',
    name: 'Gemini 2.5 Flash',
    quantizations: [],
    vision: true,
    reasoning: true,
  },
  {
    provider: 'claude',
    model: 'claude-3-7-sonnet',
    name: 'Claude 3.7 Sonnet',
    quantizations: [],
    vision: true,
    reasoning: true,
  },
  {
    provider: 'deepseek',
    model: 'deepseek-v3',
    name: 'DeepSeek V3',
    quantizations: [],
    vision: false,
    reasoning: true,
  },
]

export const MOCK_FONTS: FontFamily[] = [
  {
    name: 'CCWildWords',
    metadata: {
      primary_script: 'Latin',
      scripts: ['Latin'],
      languages: ['en', 'vi', 'ja'],
      category: 'Comic',
      classifications: ['Dialogue'],
      use_cases: ['dialogue'],
    },
    sources: ['system'],
    faces: [
      {
        postscript_name: 'CCWildWords-Roman',
        weight: 400,
        weight_range: null,
        style: 'normal',
      },
      {
        postscript_name: 'CCWildWords-Bold',
        weight: 700,
        weight_range: null,
        style: 'normal',
      },
    ],
  },
  {
    name: 'Anime Ace',
    metadata: {
      primary_script: 'Latin',
      scripts: ['Latin'],
      languages: ['en', 'vi', 'ja'],
      category: 'Comic',
      classifications: ['Dialogue'],
      use_cases: ['dialogue'],
    },
    sources: ['system'],
    faces: [
      {
        postscript_name: 'AnimeAce-Regular',
        weight: 400,
        weight_range: null,
        style: 'normal',
      },
    ],
  },
]

export const MOCK_PREFERENCES: Preferences = {
  pipeline: {
    detection: {
      model: 'koharu-layout-rfdetr-seg-2xl',
      text_threshold: 0.5,
      bubble_threshold: 0.5,
      panel_threshold: 0.5,
    },
    ocr: { model: 'paddleocr-vl-1.6' },
    inpainting: { model: 'lama' },
    translation: {
      target_language: 'en-US',
      instructions: null,
      model: {
        provider: 'local',
        model: 'qwen3.8-27b-uncensored',
        reasoning: true,
        vision: true,
      },
      generation: {
        temperature: null,
        top_p: null,
        min_p: null,
        repeat_penalty: null,
        frequency_penalty: null,
        presence_penalty: null,
        reasoning: true,
        vision: true,
      },
    },
    processor: {
      'koharu-layout-rfdetr-seg-2xl': {
        text_threshold: 0.5,
        bubble_threshold: 0.5,
        panel_threshold: 0.5,
      },
    },
  },
  providers: {
    entries: [],
  },
  typesetting: {
    font_families: ['CCWildWords', 'Anime Ace'],
  },
  mcp: {
    enabled: true,
    port: 9999,
  },
  languages: [
    { tag: 'en-US', name: 'English' },
    { tag: 'vi-VN', name: 'Tiếng Việt' },
    { tag: 'ja-JP', name: '日本語' },
    { tag: 'zh-CN', name: '简体中文' },
  ],
}

export async function handleMockCommand(command: unknown, args: unknown[]): Promise<unknown> {
  if (command === commands.getProject) {
    return MOCK_PROJECT
  }
  if (command === commands.getPages) {
    return MOCK_PAGES
  }
  if (command === commands.getPage) {
    return MOCK_PAGE
  }
  if (command === commands.getPreferences) {
    return MOCK_PREFERENCES
  }
  if (command === commands.getTranslationModels) {
    return MOCK_MODELS
  }
  if (command === commands.getFonts) {
    return MOCK_FONTS
  }
  if (command === commands.getThumbnail) {
    const pageId = String(args[0] ?? 'page-31')
    const pageNum = pageId.replace('page-', '')
    try {
      const res = await fetch(`/showcase/${pageNum}.png`)
      const buf = await res.arrayBuffer()
      return Array.from(new Uint8Array(buf))
    } catch {
      return []
    }
  }
  if (command === commands.getAgentStatus) {
    return {
      account: null,
      models: [
        { id: 'gpt-4o', name: 'GPT-4o' },
        { id: 'claude-3-7-sonnet', name: 'Claude 3.7 Sonnet' },
      ],
      config: {
        model: 'gpt-4o',
        reasoning: 'low',
      },
      running: null,
    }
  }
  if (command === commands.selectPage) {
    return { page: MOCK_PAGE, selection: { pages: [args[0]], layers: [] } }
  }
  if (command === commands.savePreferences) {
    return MOCK_PREFERENCES
  }
  if (command === commands.listProjects) {
    return [{ name: 'Momiji to Kouyou', last_modified: new Date().toISOString() }]
  }
  if (command === commands.process) {
    return 1
  }
  return null
}
