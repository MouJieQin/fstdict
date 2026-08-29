import { type as osType } from '@tauri-apps/plugin-os'

// Standardized internal key names (always stored in config, platform-agnostic)
export type ModifierKey = 'meta' | 'ctrl' | 'alt' | 'shift'
export type NormalizedKey = ModifierKey | string

// Cached platform state
let cachedIsWindows: boolean | null = null

/**
 * Synchronous Windows detection (userAgent fallback)
 * Used for initial render before Tauri OS plugin resolves
 */
export function isWindowsSync(): boolean {
  if (cachedIsWindows !== null) return cachedIsWindows
  if (typeof navigator === 'undefined') return false
  cachedIsWindows = /Windows/.test(navigator.userAgent)
  return cachedIsWindows
}

/**
 * Initialize accurate platform detection via Tauri OS plugin
 * Call once at app startup
 */
export async function initPlatformDetection(): Promise<void> {
  try {
    const os = await osType()
    cachedIsWindows = os === 'windows'
  } catch {
    // Fallback to userAgent already active
  }
}

/**
 * Normalize from KeyboardEvent.code (physical key, FOR SHORTCUT CAPTURE)
 * e.code like KeyL, Digit1, ArrowUp
 */
export function normalizeKeyFromCode(code: string): NormalizedKey | null {
  if (!code) return null
  // Letter keys: KeyA -> A, KeyL -> L
  if (code.startsWith('Key')) {
    return code.replace('Key', '')
  }
  // Digit: Digit0 -> 0
  if (code.startsWith('Digit')) {
    return code.replace('Digit', '')
  }
  // Function keys
  if (code.startsWith('F')) {
    return code
  }
  // Named special keys
  const map: Record<string, NormalizedKey> = {
    ArrowUp: 'ArrowUp',
    ArrowDown: 'ArrowDown',
    ArrowLeft: 'ArrowLeft',
    ArrowRight: 'ArrowRight',
    Space: 'Space',
    Escape: 'Esc',
    Enter: 'Enter',
    Backspace: 'Backspace',
    Delete: 'Delete',
    Tab: 'Tab',
  }
  return map[code] ?? null
}

/**
 * Normalize raw KeyboardEvent.key (legacy, only for fallback)
 */
export function normalizeKey(rawKey: string): NormalizedKey {
  const key = rawKey.toLowerCase()
  switch (key) {
    case 'meta':
    case 'os':
    case 'command':
    case 'cmd':
      return 'meta'
    case 'control':
      return 'ctrl'
    case 'alt':
    case 'option':
      return 'alt'
    case 'shift':
      return 'shift'
    case 'arrowup':
      return 'ArrowUp'
    case 'arrowdown':
      return 'ArrowDown'
    case 'arrowleft':
      return 'ArrowLeft'
    case 'arrowright':
      return 'ArrowRight'
    case ' ':
      return 'Space'
    case 'escape':
      return 'Esc'
    case 'enter':
      return 'Enter'
    case 'backspace':
      return 'Backspace'
    case 'delete':
      return 'Delete'
    case 'tab':
      return 'Tab'
    default:
      return rawKey.length === 1 ? rawKey.toUpperCase() : rawKey
  }
}

/**
 * Check if a key is a modifier key
 */
export function isModifier(key: NormalizedKey): boolean {
  return ['meta', 'ctrl', 'alt', 'shift'].includes(key as ModifierKey)
}

/**
 * Sort keys in standard order: Meta → Ctrl → Alt → Shift → Primary key
 */
export function sortHotkey(keys: NormalizedKey[]): NormalizedKey[] {
  const modifiers: ModifierKey[] = []
  const others: string[] = []

  for (const key of keys) {
    if (isModifier(key)) {
      modifiers.push(key as ModifierKey)
    } else {
      others.push(key)
    }
  }

  const modifierOrder: ModifierKey[] = ['meta', 'ctrl', 'alt', 'shift']
  modifiers.sort((a, b) => modifierOrder.indexOf(a) - modifierOrder.indexOf(b))

  return [...modifiers, ...others]
}

/**
 * Convert normalized keys to platform-specific display symbols
 * VSCode / Figma visual style
 */
export function formatHotkey(keys: NormalizedKey[]): string[] {
  const isWin = isWindowsSync()
  const symbolMap: Record<ModifierKey, { mac: string; win: string }> = {
    meta: { mac: '⌘', win: '⊞' },
    ctrl: { mac: '⌃', win: 'Ctrl' },
    alt: { mac: '⌥', win: 'Alt' },
    shift: { mac: '⇧', win: 'Shift' },
  }

  return keys.map((key) => {
    if (isModifier(key)) {
      const mod = key as ModifierKey
      return isWin ? symbolMap[mod].win : symbolMap[mod].mac
    }
    // Special key glyphs
    const specialLabels: Record<string, string> = {
      ArrowUp: '↑',
      ArrowDown: '↓',
      ArrowLeft: '←',
      ArrowRight: '→',
      Space: '␣',
      Esc: 'Esc',
      Enter: '↵',
      Backspace: '⌫',
      Delete: '⌦',
      Tab: '⇥',
    }
    return specialLabels[key] || key.toUpperCase()
  })
}
