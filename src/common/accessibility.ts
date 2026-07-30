import { invoke } from '@tauri-apps/api/core';
// import { type } from '@tauri-apps/plugin-os';

// /**
//  * Checks if the current OS is macOS
//  */
// export async function isMacOS(): Promise<boolean> {
//     try {
//         const osType = type();
//         return osType === 'macos'; // Returns true on macOS, false on 'windows' or 'linux'
//     } catch {
//         return false;
//     }
// }

/**
 * Checks if the current OS is macOS using native web engine descriptors
 */
export function isMacOS(): boolean {
    // Works flawlessly in Tauri's underlying WebKit/WebView contexts
    return /Macintosh|MacIntel|MacPPC|Mac68K/i.test(navigator.userAgent);
}


/**
 * Safe frontend wrapper for checking accessibility rights
 */
export async function checkAccessibilitySafe(): Promise<boolean> {
    const mac = isMacOS();
    if (!mac) {
        // Automatically true/allowed on non-macOS platforms since they don't use this system
        return true;
    }

    // Safe to invoke because we confirmed we are on macOS
    return await invoke<boolean>('check_accessibility');
}

/**
 * Safe frontend wrapper for requesting accessibility rights
 */
export async function requestAccessibilitySafe(): Promise<boolean> {
    const mac = isMacOS();
    if (!mac) return true;

    return await invoke<boolean>('request_accessibility');
}
