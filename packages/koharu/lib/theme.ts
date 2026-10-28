'use client'

export interface ThemeColors {
  [key: string]: string
}

export interface ThemeDefinition {
  id: string
  name: string
  author?: string
  version?: string
  description?: string
  dark?: ThemeColors
  light?: ThemeColors
}

export const BUILTIN_THEMES: ThemeDefinition[] = [
  {
    id: 'nahida-sage',
    name: 'Nahida Sage (Mặc định)',
    author: 'chiraitori',
    version: '1.0',
    description: 'Tông xanh lá ngọc bích hài hòa cùng icon Nahida.',
    light: {
      '--primary': '#2d6a4f',
      '--primary-foreground': '#ffffff',
      '--accent': '#e8f5ee',
      '--accent-foreground': '#1b4332',
      '--ring': '#2d6a4f',
      '--surface-titlebar': '#f0f7f3',
      '--surface-sidebar': '#f0f7f3',
      '--border': '#d8e8de',
      '--canvas-selection': '#2d6a4f',
    },
    dark: {
      '--primary': '#52b788',
      '--primary-foreground': '#081c12',
      '--accent': '#183526',
      '--accent-foreground': '#b7e4c7',
      '--ring': '#52b788',
      '--surface-titlebar': '#0c1410',
      '--surface-toolbar': '#070c09',
      '--surface-sidebar': '#0c1410',
      '--surface-panel': '#0e1713',
      '--surface-floating': '#14211b',
      '--surface-well': '#111c17',
      '--surface-canvas': '#070b09',
      '--background': '#070b09',
      '--border': '#1b2922',
      '--canvas-selection': '#52b788',
    },
  },
  {
    id: 'default',
    name: 'Koharu Pink (Gốc)',
    author: 'Mayo Takanashi',
    version: '1.0',
    description: 'Tông màu hồng hoa anh đào nguyên bản.',
    light: {},
    dark: {},
  },
  {
    id: 'midnight-obsidian',
    name: 'Midnight Obsidian (Đen Tuyền)',
    author: 'chiraitori',
    version: '1.0',
    description: 'Chế độ tối AMOLED sâu thẳm, viền xám tinh tế.',
    dark: {
      '--primary': '#ffffff',
      '--primary-foreground': '#000000',
      '--background': '#000000',
      '--surface-titlebar': '#050505',
      '--surface-toolbar': '#000000',
      '--surface-sidebar': '#050505',
      '--surface-panel': '#080808',
      '--surface-floating': '#0f0f0f',
      '--surface-well': '#121212',
      '--surface-canvas': '#000000',
      '--border': '#222222',
      '--input': '#333333',
      '--accent': '#1c1c1c',
      '--accent-foreground': '#ffffff',
      '--ring': '#ffffff',
      '--canvas-selection': '#ffffff',
    },
  },
  {
    id: 'cyber-neon',
    name: 'Cyber Neon (Tím & Cyan)',
    author: 'chiraitori',
    version: '1.0',
    description: 'Phong cách tương lai với nền tím thẫm và điểm nhấn neon cyan.',
    dark: {
      '--primary': '#00f5d4',
      '--primary-foreground': '#041d19',
      '--accent': '#2b0938',
      '--accent-foreground': '#f72585',
      '--ring': '#00f5d4',
      '--background': '#0d0214',
      '--surface-titlebar': '#13031d',
      '--surface-toolbar': '#0a0110',
      '--surface-sidebar': '#13031d',
      '--surface-panel': '#160522',
      '--surface-floating': '#200731',
      '--surface-well': '#1a0528',
      '--surface-canvas': '#0d0214',
      '--border': '#341145',
      '--input': '#48175f',
      '--destructive': '#f72585',
      '--canvas-selection': '#00f5d4',
    },
  },
]

const ACTIVE_THEME_KEY = 'kore-minit-active-theme-id'
const CUSTOM_THEMES_KEY = 'kore-minit-custom-themes'
const STYLE_ELEMENT_ID = 'kore-custom-theme-style'

export function getCustomThemes(): ThemeDefinition[] {
  if (typeof window === 'undefined') return []
  try {
    const raw = localStorage.getItem(CUSTOM_THEMES_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

export function getAllThemes(): ThemeDefinition[] {
  return [...BUILTIN_THEMES, ...getCustomThemes()]
}

export function getActiveThemeId(): string {
  if (typeof window === 'undefined') return 'nahida-sage'
  return localStorage.getItem(ACTIVE_THEME_KEY) || 'nahida-sage'
}

export function getActiveTheme(): ThemeDefinition {
  const activeId = getActiveThemeId()
  const all = getAllThemes()
  return all.find((t) => t.id === activeId) || BUILTIN_THEMES[0]
}

export function applyTheme(theme: ThemeDefinition): void {
  if (typeof document === 'undefined') return

  let styleEl = document.getElementById(STYLE_ELEMENT_ID) as HTMLStyleElement | null
  if (!styleEl) {
    styleEl = document.createElement('style')
    styleEl.id = STYLE_ELEMENT_ID
    document.head.appendChild(styleEl)
  }

  let css = ''

  if (theme.light && Object.keys(theme.light).length > 0) {
    css += `:root {\n`
    for (const [key, val] of Object.entries(theme.light)) {
      const varName = key.startsWith('--') ? key : `--${key}`
      css += `  ${varName}: ${val} !important;\n`
    }
    css += `}\n`
  }

  if (theme.dark && Object.keys(theme.dark).length > 0) {
    css += `.dark {\n`
    for (const [key, val] of Object.entries(theme.dark)) {
      const varName = key.startsWith('--') ? key : `--${key}`
      css += `  ${varName}: ${val} !important;\n`
    }
    css += `}\n`
  }

  styleEl.textContent = css
}

export function setActiveTheme(id: string): void {
  if (typeof window === 'undefined') return
  localStorage.setItem(ACTIVE_THEME_KEY, id)
  const theme = getAllThemes().find((t) => t.id === id)
  if (theme) {
    applyTheme(theme)
  }
}

export function saveCustomTheme(theme: ThemeDefinition): void {
  if (typeof window === 'undefined') return
  const customs = getCustomThemes().filter((t) => t.id !== theme.id)
  customs.push(theme)
  localStorage.setItem(CUSTOM_THEMES_KEY, JSON.stringify(customs))
  setActiveTheme(theme.id)
}

export function deleteCustomTheme(id: string): void {
  if (typeof window === 'undefined') return
  const customs = getCustomThemes().filter((t) => t.id !== id)
  localStorage.setItem(CUSTOM_THEMES_KEY, JSON.stringify(customs))
  if (getActiveThemeId() === id) {
    setActiveTheme('nahida-sage')
  }
}

export function initTheme(): void {
  if (typeof window === 'undefined') return
  const theme = getActiveTheme()
  applyTheme(theme)
}

export function exportThemeJson(theme: ThemeDefinition): string {
  const exportData = {
    name: theme.name,
    author: theme.author || 'User',
    version: theme.version || '1.0',
    description: theme.description || '',
    light: theme.light || {},
    dark: theme.dark || {},
  }
  return JSON.stringify(exportData, null, 2)
}

export function parseThemeJson(jsonString: string): ThemeDefinition {
  const obj = JSON.parse(jsonString)
  if (typeof obj !== 'object' || obj === null) {
    throw new Error('File JSON không hợp lệ.')
  }
  if (!obj.name || typeof obj.name !== 'string') {
    throw new Error("Thiếu trường 'name' trong theme JSON.")
  }

  const id = 'custom-' + Date.now() + '-' + Math.random().toString(36).substring(2, 7)
  return {
    id,
    name: obj.name,
    author: typeof obj.author === 'string' ? obj.author : 'User',
    version: typeof obj.version === 'string' ? obj.version : '1.0',
    description: typeof obj.description === 'string' ? obj.description : '',
    light: typeof obj.light === 'object' && obj.light !== null ? obj.light : {},
    dark: typeof obj.dark === 'object' && obj.dark !== null ? obj.dark : {},
  }
}
