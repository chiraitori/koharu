import type { SVGProps } from 'react'

/**
 * Official OpenCode brand icon (authentic dark emblem with geometric bracket)
 * Source: anomalyco/opencode (packages/ui/src/assets/favicon/favicon.svg)
 */
export function OpenCodeIcon({ className = 'size-4', ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox='0 0 512 512'
      fill='none'
      xmlns='http://www.w3.org/2000/svg'
      className={className}
      {...props}
    >
      <title>OpenCode</title>
      <rect width='512' height='512' rx='96' fill='#131010' />
      <path d='M320 224V352H192V224H320Z' fill='#5A5858' />
      <path
        fillRule='evenodd'
        clipRule='evenodd'
        d='M384 416H128V96H384V416ZM320 160H192V352H320V160Z'
        fill='white'
      />
    </svg>
  )
}
