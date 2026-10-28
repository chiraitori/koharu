'use client'

import { Code, Download, Plus, Trash2, Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  exportThemeJson,
  getActiveTheme,
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

  const handleSelectTheme = (id: string) => {
    setActiveId(id)
    setActiveTheme(id)
    toast.add({
      type: 'info',
      title: 'Đã đổi theme',
      description: `Đang áp dụng: ${themes.find((t) => t.id === id)?.name || id}`,
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
          title: 'Nhập theme thành công',
          description: `Đã cài đặt theme: ${parsed.name}`,
        })
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : 'Lỗi không xác định'
        toast.add({
          type: 'error',
          title: 'Không thể đọc file theme',
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
        title: 'Xuất theme thành công',
        description: `Đã tải về file ${fileName}`,
      })
    } catch {
      toast.add({
        type: 'error',
        title: 'Lỗi xuất file',
        description: 'Không thể tạo file JSON cho theme này.',
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
        description: 'Tùy biến màu sắc cá nhân.',
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
        title: 'Đã lưu theme',
        description: `Theme "${parsed.name}" đã được lưu và kích hoạt.`,
      })
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Cú pháp JSON không hợp lệ'
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
      title: 'Đã xóa theme',
      description: 'Theme tùy biến đã được xóa.',
    })
  }

  return (
    <>
      <PreferenceRow
        title='Bảng màu & Theme JSON'
        description='Chọn bảng màu có sẵn hoặc tùy biến màu sắc giao diện bằng file JSON.'
      >
        <div className='flex flex-col gap-2'>
          <div className='flex items-center gap-2'>
            <Select
              value={activeId}
              items={Object.fromEntries(themes.map((th) => [th.id, th.name]))}
              onValueChange={(val) => val && handleSelectTheme(val)}
            >
              <SelectTrigger className='h-8 w-full text-[11px]'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {themes.map((th) => (
                  <SelectItem key={th.id} value={th.id}>
                    {th.name}
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
                title='Xóa theme này'
                onClick={() => handleDelete(activeId)}
              >
                <Trash2 className='size-3.5' />
              </Button>
            )}
          </div>

          {activeTheme.description && (
            <p className='text-[10px] text-muted-foreground italic'>{activeTheme.description}</p>
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
              <Upload className='size-3' /> Nhập JSON
            </Button>

            <Button
              type='button'
              variant='outline'
              size='sm'
              className='h-7 gap-1 text-[10px]'
              onClick={handleExport}
            >
              <Download className='size-3' /> Xuất JSON
            </Button>

            <Button
              type='button'
              variant='outline'
              size='sm'
              className='h-7 gap-1 text-[10px]'
              onClick={() => handleOpenEditor(false)}
            >
              <Code className='size-3' /> Sửa JSON
            </Button>

            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-7 gap-1 text-[10px] text-muted-foreground hover:text-foreground'
              onClick={() => handleOpenEditor(true)}
            >
              <Plus className='size-3' /> Tạo mới
            </Button>
          </div>
        </div>
      </PreferenceRow>

      <Dialog open={editorOpen} onOpenChange={setEditorOpen}>
        <DialogContent className='max-w-md gap-3 p-5'>
          <DialogHeader className='gap-1'>
            <DialogTitle className='text-[14px] font-semibold'>Chỉnh sửa Theme JSON</DialogTitle>
            <DialogDescription className='text-[11px] text-muted-foreground'>
              Tùy biến các biến màu CSS trong các mục <code>light</code> và <code>dark</code>.
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
              Hủy
            </Button>
            <Button
              type='button'
              variant='default'
              size='sm'
              className='text-[11px]'
              onClick={handleSaveEditor}
            >
              Áp dụng & Lưu
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
