'use client'

import { Code, Download, Plus, Trash2, Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  exportThemeJson,
  getActiveThemeId,
  getAllThemes,
  parseThemeJson,
  saveCustomTheme,
  setActiveTheme,
  deleteCustomTheme,
  type ThemeDefinition,
  BUILTIN_THEMES,
} from '@/lib/theme'
import { Button } from '@koharu/ui/components/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@koharu/ui/components/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@koharu/ui/components/select'
import { Textarea } from '@koharu/ui/components/textarea'
import { toast } from '@koharu/ui/components/toast'
import { PreferenceRow } from '@/components/preferences/PreferenceFields'

export function ThemePreferences() {
  const { t } = useTranslation()
  const [activeId, setActiveId] = useState(getActiveThemeId())
  const [themes, setThemes] = useState(getAllThemes())
  const [editorOpen, setEditorOpen] = useState(false)
  const [editorJson, setEditorJson] = useState('')
  const [editorError, setEditorError] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement | null>(null)

  const activeTheme = themes.find((th) => th.id === activeId) || themes[0]
  const isCustom = !BUILTIN_THEMES.some((b) => b.id === activeId)

  const getThemeDisplayName = (th: ThemeDefinition) => {
    const isBuiltin = BUILTIN_THEMES.some((b) => b.id === th.id)
    return isBuiltin ? t(`settings.appearance.themePresets.${th.id}`, th.name) : th.name
  }

  const getThemeDisplayDescription = (th: ThemeDefinition) => {
    const isBuiltin = BUILTIN_THEMES.some((b) => b.id === th.id)
    return isBuiltin
      ? t(`settings.appearance.themePresets.${th.id}Desc`, th.description || '')
      : th.description
  }

  const handleSelectTheme = (id: string) => {
    setActiveId(id)
    setActiveTheme(id)
    const target = themes.find((th) => th.id === id)
    const displayName = target ? getThemeDisplayName(target) : id
    toast.add({
      type: 'info',
      title: t('settings.appearance.toasts.themeChanged'),
      description: t('settings.appearance.toasts.applying', { name: displayName }),
    })
  }

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    const reader = new FileReader()
    reader.onload = (event) => {
      try {
        const content = event.target?.result as string
        const parsed = parseThemeJson(content)
        saveCustomTheme(parsed)
        const updated = getAllThemes()
        setThemes(updated)
        setActiveId(parsed.id)
        toast.add({
          type: 'success',
          title: t('settings.appearance.toasts.importSuccess'),
          description: t('settings.appearance.toasts.importSuccessDesc', { name: parsed.name }),
        })
      } catch (err: unknown) {
        const msg =
          err instanceof Error ? err.message : t('settings.appearance.toasts.invalidJson')
        toast.add({
          type: 'error',
          title: t('settings.appearance.toasts.importError'),
          description: msg,
        })
      } finally {
        if (fileInputRef.current) fileInputRef.current.value = ''
      }
    }
    reader.readAsText(file)
  }

  const handleExport = () => {
    try {
      const jsonStr = exportThemeJson(activeTheme)
      const blob = new Blob([jsonStr], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      const fileName = `${activeTheme.name.toLowerCase().replace(/[^a-z0-9_-]/g, '_')}_theme.json`
      a.href = url
      a.download = fileName
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      toast.add({
        type: 'success',
        title: t('settings.appearance.toasts.exportSuccess'),
        description: t('settings.appearance.toasts.exportSuccessDesc', { file: fileName }),
      })
    } catch {
      toast.add({
        type: 'error',
        title: t('settings.appearance.toasts.exportError'),
        description: 'Failed to export JSON file.',
      })
    }
  }

  const handleOpenEditor = (createNew = false) => {
    if (createNew) {
      const template: ThemeDefinition = {
        id: 'new',
        name: 'My Custom Theme',
        author: 'User',
        version: '1.0',
        description: 'Custom palette created by user.',
        light: {
          '--primary': '#2d6a4f',
          '--primary-foreground': '#ffffff',
          '--accent': '#e8f5ee',
          '--accent-foreground': '#1b4332',
          '--surface-titlebar': '#f0f7f3',
        },
        dark: {
          '--primary': '#52b788',
          '--primary-foreground': '#081c12',
          '--accent': '#183526',
          '--accent-foreground': '#b7e4c7',
          '--surface-titlebar': '#0c1410',
          '--background': '#070b09',
        },
      }
      setEditorJson(exportThemeJson(template))
    } else {
      setEditorJson(exportThemeJson(activeTheme))
    }
    setEditorError(null)
    setEditorOpen(true)
  }

  const handleSaveEditor = () => {
    try {
      const parsed = parseThemeJson(editorJson)
      saveCustomTheme(parsed)
      const updated = getAllThemes()
      setThemes(updated)
      setActiveId(parsed.id)
      setEditorOpen(false)
      toast.add({
        type: 'success',
        title: t('settings.appearance.toasts.saveSuccess'),
        description: t('settings.appearance.toasts.saveSuccessDesc', { name: parsed.name }),
      })
    } catch (err: unknown) {
      const msg =
        err instanceof Error ? err.message : t('settings.appearance.toasts.invalidJson')
      setEditorError(msg)
    }
  }

  const handleDelete = (id: string) => {
    deleteCustomTheme(id)
    const updated = getAllThemes()
    setThemes(updated)
    const fallbackId = getActiveThemeId()
    setActiveId(fallbackId)
    toast.add({
      type: 'info',
      title: t('settings.appearance.toasts.deleteSuccess'),
      description: t('settings.appearance.toasts.deleteSuccessDesc'),
    })
  }

  return (
    <>
      <PreferenceRow
        title={t('settings.appearance.customTheme')}
        description={t('settings.appearance.customThemeDescription')}
      >
        <div className='flex flex-col gap-2'>
          <div className='flex items-center gap-2'>
            <Select
              value={activeId}
              items={Object.fromEntries(themes.map((th) => [th.id, getThemeDisplayName(th)]))}
              onValueChange={(val) => val && handleSelectTheme(val)}
            >
              <SelectTrigger className='h-8 w-full text-[11px]'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {themes.map((th) => (
                  <SelectItem key={th.id} value={th.id}>
                    {getThemeDisplayName(th)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            {isCustom && (
              <Button
                type='button'
                variant='destructive'
                size='icon-sm'
                className='size-8 shrink-0'
                title={t('settings.appearance.deleteTheme')}
                onClick={() => handleDelete(activeId)}
              >
                <Trash2 className='size-3.5' />
              </Button>
            )}
          </div>

          {getThemeDisplayDescription(activeTheme) && (
            <p className='text-[10px] text-muted-foreground italic'>
              {getThemeDisplayDescription(activeTheme)}
            </p>
          )}

          <div className='flex flex-wrap items-center gap-1.5 pt-1'>
            <input
              type='file'
              ref={fileInputRef}
              accept='.json'
              className='hidden'
              onChange={handleFileUpload}
            />

            <Button
              type='button'
              variant='outline'
              size='sm'
              className='h-7 gap-1 text-[10px]'
              onClick={() => fileInputRef.current?.click()}
            >
              <Upload className='size-3' /> {t('settings.appearance.importJson')}
            </Button>

            <Button
              type='button'
              variant='outline'
              size='sm'
              className='h-7 gap-1 text-[10px]'
              onClick={handleExport}
            >
              <Download className='size-3' /> {t('settings.appearance.exportJson')}
            </Button>

            <Button
              type='button'
              variant='outline'
              size='sm'
              className='h-7 gap-1 text-[10px]'
              onClick={() => handleOpenEditor(false)}
            >
              <Code className='size-3' /> {t('settings.appearance.editJson')}
            </Button>

            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-7 gap-1 text-[10px] text-muted-foreground hover:text-foreground'
              onClick={() => handleOpenEditor(true)}
            >
              <Plus className='size-3' /> {t('settings.appearance.createTheme')}
            </Button>
          </div>
        </div>
      </PreferenceRow>

      <Dialog open={editorOpen} onOpenChange={setEditorOpen}>
        <DialogContent className='max-w-md gap-3 p-5'>
          <DialogHeader className='gap-1'>
            <DialogTitle className='text-[14px] font-semibold'>
              {t('settings.appearance.editorTitle')}
            </DialogTitle>
            <DialogDescription className='text-[11px] text-muted-foreground'>
              {t('settings.appearance.editorDescription')}
            </DialogDescription>
          </DialogHeader>

          <div className='flex flex-col gap-2'>
            <Textarea
              className='h-64 font-mono text-[10px] leading-relaxed resize-none'
              value={editorJson}
              onChange={(e) => {
                setEditorJson(e.target.value)
                setEditorError(null)
              }}
              placeholder='{\n  "name": "My Theme",\n  "dark": { "--primary": "#52b788" }\n}'
              spellCheck={false}
            />

            {editorError && (
              <p className='text-[10px] font-medium text-destructive'>{editorError}</p>
            )}
          </div>

          <DialogFooter className='gap-2 sm:justify-end'>
            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='text-[11px]'
              onClick={() => setEditorOpen(false)}
            >
              {t('settings.appearance.cancel')}
            </Button>
            <Button
              type='button'
              variant='default'
              size='sm'
              className='text-[11px]'
              onClick={handleSaveEditor}
            >
              {t('settings.appearance.applyAndSave')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
