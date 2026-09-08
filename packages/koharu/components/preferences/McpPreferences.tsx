'use client'

import { Check, Copy } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { AntigravityIcon } from '@/components/icons/AntigravityIcon'
import { ClaudeCodeIcon } from '@/components/icons/ClaudeCodeIcon'
import { ClaudeIcon } from '@/components/icons/ClaudeIcon'
import { CodexIcon } from '@/components/icons/CodexIcon'
import { OpenCodeIcon } from '@/components/icons/OpenCodeIcon'
import {
  NumberField,
  PreferencePage,
  PreferenceRow,
  PreferenceSection,
} from '@/components/preferences/PreferenceFields'
import type { McpConfig } from '@koharu/bridge/protocol'
import { Badge } from '@koharu/ui/components/badge'
import { Button } from '@koharu/ui/components/button'
import { Switch } from '@koharu/ui/components/switch'

const TOOLS_SUMMARY = [
  {
    name: 'inspect_project',
    desc: 'Read the latest semantic state of the open project: pages, layers, translations, fonts, pipeline settings.',
  },
  {
    name: 'view_page',
    desc: 'Render and return a page image (Base64 WebP) for visual inspection and bubble localization.',
  },
  {
    name: 'import_images',
    desc: 'Import image files, directories, ZIP/CBZ/RAR archives, or PDF files into the project.',
  },
  {
    name: 'run_pipeline',
    desc: 'Trigger processing stages (detect, OCR, translation, inpainting) for the project, pages, or elements.',
  },
  {
    name: 'set_translation',
    desc: 'Update translated text in speech bubbles directly, or pass null to clear it.',
  },
  {
    name: 'set_source_text',
    desc: 'Correct recognized source (OCR) text.',
  },
  {
    name: 'add_text_box',
    desc: 'Add a new speech bubble with custom boundary coordinates.',
  },
  {
    name: 'set_typography',
    desc: 'Customize font family, size, weight, style, alignment, and colors.',
  },
  {
    name: 'set_geometry',
    desc: 'Replace an element\u2019s page-space polygon geometry points.',
  },
  {
    name: 'set_visibility',
    desc: 'Change visibility and/or opacity for one or more elements.',
  },
  {
    name: 'delete_elements',
    desc: 'Delete elements and their descendants from the scene.',
  },
  {
    name: 'rename_page',
    desc: 'Rename a project page.',
  },
  {
    name: 'move_page',
    desc: 'Move a page to a new zero-based index in the page list.',
  },
  {
    name: 'delete_pages',
    desc: 'Delete pages and their contents from the project.',
  },
  {
    name: 'open_project',
    desc: 'Open an existing project from the library by name.',
  },
  {
    name: 'create_project',
    desc: 'Create and open a new empty project in the library.',
  },
  {
    name: 'list_projects',
    desc: 'List all projects in the library.',
  },
  {
    name: 'move_element',
    desc: 'Move an element under a page or element at a zero-based index.',
  },
  {
    name: 'undo / redo',
    desc: 'Undo or redo the last action in the active project.',
  },
  {
    name: 'select_page',
    desc: 'Select the active page by entity ID.',
  },
  {
    name: 'close_project / delete_project',
    desc: 'Close the open project, or permanently delete a project by name.',
  },
  {
    name: 'export_pages',
    desc: 'Export pages to PNG or PSD files in an existing directory.',
  },
  {
    name: 'add_point_text',
    desc: 'Add point text on the active page at x/y in page space.',
  },
  {
    name: 'commit_paint / commit_erase',
    desc: 'Paint a raster stroke (creates a layer when omitted) or erase part of a paint layer.',
  },
  {
    name: 'commit_transform',
    desc: 'Move, resize, or rotate text elements by giving new frames in page space.',
  },
  {
    name: 'request_inpaint',
    desc: 'Run inpainting for a brushed region on a page. Returns the pipeline job ID.',
  },
  {
    name: 'stop_processing',
    desc: 'Stop all running pipeline jobs.',
  },
  {
    name: 'get_fonts / get_translation_models / get_preferences',
    desc: 'List available fonts, translation models, and current preferences.',
  },
  {
    name: 'get_page_thumbnail',
    desc: 'Get a small WebP thumbnail of a page source image.',
  },
]

export function McpPreferences({
  value,
  onChange,
}: {
  value: McpConfig
  onChange: (value: McpConfig) => void
}) {
  const { t } = useTranslation()
  const [copiedKey, setCopiedKey] = useState<string | null>(null)

  const port = value.port || 9999
  const mcpUrl = `http://127.0.0.1:${port}/mcp`

  const copyToClipboard = (key: string, text: string) => {
    void navigator.clipboard.writeText(text)
    setCopiedKey(key)
    window.setTimeout(() => {
      setCopiedKey((current) => (current === key ? null : current))
    }, 2000)
  }

  const antigravityConfig = JSON.stringify(
    {
      mcpServers: {
        koharu: {
          serverUrl: mcpUrl,
        },
      },
    },
    null,
    2,
  )

  const claudeDesktopConfig = JSON.stringify(
    {
      mcpServers: {
        koharu: {
          command: 'npx',
          args: ['-y', 'mcp-remote@latest', mcpUrl],
        },
      },
    },
    null,
    2,
  )

  const claudeCodeCli = `claude mcp add-json koharu "{\\"type\\":\\"stdio\\",\\"command\\":\\"npx\\",\\"args\\":[\\"-y\\",\\"mcp-remote@latest\\",\\"${mcpUrl}\\"],\\"env\\":{}}"`

  const codexConfig = `[mcp_servers.koharu]
url = "${mcpUrl}"`

  const openCodeConfig = JSON.stringify(
    {
      $schema: 'https://opencode.ai/config.json',
      mcp: {
        koharu: {
          type: 'remote',
          url: mcpUrl,
          enabled: true,
        },
      },
    },
    null,
    2,
  )

  return (
    <PreferencePage
      title={t('settings.mcp.title', 'Model Context Protocol (MCP)')}
      description={t(
        'settings.mcp.description',
        'Expose Koharu tools over local Streamable HTTP / SSE for AI client automation (Google Antigravity, Claude Desktop, Cursor, OpenCode).',
      )}
    >
      <PreferenceSection
        title={t('settings.mcp.service', 'Server Configuration')}
        description={t(
          'settings.mcp.serviceDesc',
          'Enable the built-in MCP server to let external AI models inspect, translate, and automate your manga projects.',
        )}
      >
        <PreferenceRow
          title={t('settings.mcp.enableTitle', 'Enable MCP Server')}
          description={t(
            'settings.mcp.enableDesc',
            'Runs an embedded MCP server listening on localhost.',
          )}
        >
          <div className='flex items-center gap-3'>
            <Switch
              checked={value.enabled}
              onCheckedChange={(checked) => onChange({ ...value, enabled: checked })}
            />
            {value.enabled ? (
              <Badge className='border-emerald-500/30 bg-emerald-500/15 text-[10px] font-medium text-emerald-600 dark:text-emerald-400'>
                {t('settings.mcp.active', 'Running')}
              </Badge>
            ) : (
              <Badge variant='outline' className='text-[10px] text-muted-foreground'>
                {t('settings.mcp.inactive', 'Disabled')}
              </Badge>
            )}
          </div>
        </PreferenceRow>

        <PreferenceRow
          title={t('settings.mcp.portTitle', 'Server Port')}
          description={t(
            'settings.mcp.portDesc',
            'Local TCP port for the MCP endpoint (default: 9999).',
          )}
        >
          <div className='w-32'>
            <NumberField
              label={t('settings.mcp.portLabel', 'Port')}
              value={value.port}
              min={1024}
              max={65535}
              onChange={(nextPort) => onChange({ ...value, port: nextPort ?? 9999 })}
            />
          </div>
        </PreferenceRow>

        <PreferenceRow
          title={t('settings.mcp.endpointTitle', 'Endpoint URL')}
          description={t('settings.mcp.endpointDesc', 'Standard HTTP/SSE endpoint for clients.')}
        >
          <div className='flex items-center gap-2'>
            <code className='rounded bg-muted/80 px-2 py-1 font-mono text-[11px] text-foreground'>
              {mcpUrl}
            </code>
            <Button
              type='button'
              variant='outline'
              size='sm'
              className='h-7 gap-1.5 px-2 text-[11px]'
              onClick={() => copyToClipboard('url', mcpUrl)}
            >
              {copiedKey === 'url' ? (
                <>
                  <Check className='size-3 text-emerald-600' />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className='size-3' />
                  <span>Copy URL</span>
                </>
              )}
            </Button>
          </div>
        </PreferenceRow>
      </PreferenceSection>

      <PreferenceSection
        title={t('settings.mcp.clientsTitle', 'Client Setup & Snippets')}
        description={t(
          'settings.mcp.clientsDesc',
          'Add one of these configurations to your preferred AI assistant to connect it to Koharu.',
        )}
      >
        {/* Antigravity Setup */}
        <div className='space-y-2 py-3.5'>
          <div className='flex items-center justify-between'>
            <div className='flex items-center gap-2'>
              <AntigravityIcon className='size-3.5' />
              <span className='text-[12px] font-medium'>Google Antigravity</span>
              <span className='text-[10px] text-muted-foreground'>
                (~/.gemini/antigravity/mcp_config.json)
              </span>
            </div>
            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-6 gap-1 px-2 text-[11px]'
              onClick={() => copyToClipboard('antigravity', antigravityConfig)}
            >
              {copiedKey === 'antigravity' ? (
                <>
                  <Check className='size-3 text-emerald-600' />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className='size-3' />
                  <span>Copy JSON</span>
                </>
              )}
            </Button>
          </div>
          <pre className='overflow-x-auto rounded-lg border border-border/50 bg-muted/60 p-3 font-mono text-[11px] leading-relaxed text-foreground/90'>
            {antigravityConfig}
          </pre>
        </div>

        {/* Claude Desktop Setup */}
        <div className='space-y-2 border-t border-border/60 py-3.5'>
          <div className='flex items-center justify-between'>
            <div className='flex items-center gap-2'>
              <ClaudeIcon className='size-3.5' />
              <span className='text-[12px] font-medium'>Claude Desktop</span>
              <span className='text-[10px] text-muted-foreground'>
                (claude_desktop_config.json)
              </span>
            </div>
            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-6 gap-1 px-2 text-[11px]'
              onClick={() => copyToClipboard('claude', claudeDesktopConfig)}
            >
              {copiedKey === 'claude' ? (
                <>
                  <Check className='size-3 text-emerald-600' />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className='size-3' />
                  <span>Copy JSON</span>
                </>
              )}
            </Button>
          </div>
          <pre className='overflow-x-auto rounded-lg border border-border/50 bg-muted/60 p-3 font-mono text-[11px] leading-relaxed text-foreground/90'>
            {claudeDesktopConfig}
          </pre>
        </div>

        {/* Claude Code CLI Setup */}
        <div className='space-y-2 border-t border-border/60 py-3.5'>
          <div className='flex items-center justify-between'>
            <div className='flex items-center gap-2'>
              <ClaudeCodeIcon className='size-3.5' />
              <span className='text-[12px] font-medium'>Claude Code CLI</span>
            </div>
            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-6 gap-1 px-2 text-[11px]'
              onClick={() => copyToClipboard('cli', claudeCodeCli)}
            >
              {copiedKey === 'cli' ? (
                <>
                  <Check className='size-3 text-emerald-600' />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className='size-3' />
                  <span>Copy Command</span>
                </>
              )}
            </Button>
          </div>
          <pre className='overflow-x-auto rounded-lg border border-border/50 bg-muted/60 p-3 font-mono text-[11px] leading-relaxed text-foreground/90'>
            {claudeCodeCli}
          </pre>
        </div>

        {/* OpenAI Codex Setup */}
        <div className='space-y-2 border-t border-border/60 py-3.5'>
          <div className='flex items-center justify-between'>
            <div className='flex items-center gap-2'>
              <CodexIcon className='size-3.5' />
              <span className='text-[12px] font-medium'>OpenAI Codex</span>
              <span className='text-[10px] text-muted-foreground'>
                (~/.codex/config.toml)
              </span>
            </div>
            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-6 gap-1 px-2 text-[11px]'
              onClick={() => copyToClipboard('codex', codexConfig)}
            >
              {copiedKey === 'codex' ? (
                <>
                  <Check className='size-3 text-emerald-600' />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className='size-3' />
                  <span>Copy TOML</span>
                </>
              )}
            </Button>
          </div>
          <pre className='overflow-x-auto rounded-lg border border-border/50 bg-muted/60 p-3 font-mono text-[11px] leading-relaxed text-foreground/90'>
            {codexConfig}
          </pre>
        </div>

        {/* OpenCode Setup */}
        <div className='space-y-2 border-t border-border/60 py-3.5'>
          <div className='flex items-center justify-between'>
            <div className='flex items-center gap-2'>
              <OpenCodeIcon className='size-3.5' />
              <span className='text-[12px] font-medium'>OpenCode</span>
              <span className='text-[10px] text-muted-foreground'>
                (~/.config/opencode/opencode.json)
              </span>
            </div>
            <Button
              type='button'
              variant='ghost'
              size='sm'
              className='h-6 gap-1 px-2 text-[11px]'
              onClick={() => copyToClipboard('opencode', openCodeConfig)}
            >
              {copiedKey === 'opencode' ? (
                <>
                  <Check className='size-3 text-emerald-600' />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className='size-3' />
                  <span>Copy JSON</span>
                </>
              )}
            </Button>
          </div>
          <pre className='overflow-x-auto rounded-lg border border-border/50 bg-muted/60 p-3 font-mono text-[11px] leading-relaxed text-foreground/90'>
            {openCodeConfig}
          </pre>
        </div>
      </PreferenceSection>

      <PreferenceSection
        title={t('settings.mcp.toolsTitle', 'Available Automation Tools')}
        description={t(
          'settings.mcp.toolsDesc',
          'Tools exposed to connected AI models for end-to-end translation and scene editing.',
        )}
      >
        <div className='divide-y divide-border/40'>
          {TOOLS_SUMMARY.map((tool) => (
            <div key={tool.name} className='flex flex-col gap-0.5 py-2.5'>
              <div className='flex items-center gap-2'>
                <code className='font-mono text-[11px] font-semibold text-primary'>
                  {tool.name}
                </code>
              </div>
              <p className='text-[11px] leading-4 text-muted-foreground'>{tool.desc}</p>
            </div>
          ))}
        </div>
      </PreferenceSection>
    </PreferencePage>
  )
}
