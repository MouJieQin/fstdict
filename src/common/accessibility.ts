import { invoke } from '@tauri-apps/api/core'

/**
 * Detects macOS from the user agent string.
 * Works reliably in Tauri WebKit and standard browser contexts.
 */
export function isMacOS(): boolean {
    return /Macintosh|MacIntel|MacPPC|Mac68K/i.test(navigator.userAgent)
}

/**
 * Checks whether accessibility permissions are already granted.
 * Returns true on non-macOS platforms since they don't require it.
 */
export async function checkAccessibilitySafe(): Promise<boolean> {
    if (!isMacOS()) return true

    try {
        return await invoke<boolean>('check_accessibility')
    } catch {
        return false
    }
}

/**
 * Requests accessibility permissions from the system.
 * No-op on non-macOS platforms.
 */
export async function requestAccessibilitySafe(): Promise<boolean> {
    if (!isMacOS()) return true

    try {
        return await invoke<boolean>('request_accessibility')
    } catch {
        return false
    }
}