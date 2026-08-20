import type { DictSettingInfo, SessionConfig } from '@/common/type-interface'
import { useDictConfigStore } from '@/stores/dictConfig'
import { useRoute } from 'vue-router'
import { toRaw } from 'vue'

/**
 * Returns the env from route parameters.
 */
export function env(): string {
    return (useRoute().query.env as string) || ''
}

// ─── Safe deep clone helper ─────────────────────────────────────
export function safeDeepClone<T>(value: T): T {
    try {
        // Unwrap Vue reactive proxy first, then structuredClone
        return structuredClone(toRaw(value))
    } catch {
        // Fallback for objects that still can't be structured-cloned
        return JSON.parse(JSON.stringify(value))
    }
}

/**
 * Returns an ordered list of enabled dictionary names for lookup.
 */
export function getDictSettingsForLookup(optionName: string): string[] {
    const store = useDictConfigStore()
    const options = store.dictConfig?.dict_set_options?.[optionName]

    if (!options) return []

    return options
        .filter((item: DictSettingInfo) => item.is_enabled)
        .map((item) => item.name)
}

/**
 * Creates a default session configuration object.
 */
export function getDefaultSessionConfig(sessionName: string): SessionConfig {
    return {
        name: sessionName,
        default_folder: { id: null },
        dict_setting_option_name: 'default',
        default_search_method: { method: 'prefix_search' },
        ocr_lang_type: 'English',
        pin: { is_pinned: false },
    }
}

/**
 * Determines whether a regex pattern requires scanning all FST nodes.
 *
 * The engine implicitly anchors patterns to the start of the string.
 * Returns true if there is no literal prefix before the first variable-length
 * or alternation meta-character, meaning the automaton cannot prune any paths.
 */
export function willScanAllFstNodes(pattern: string): boolean {
    if (!pattern) return true

    let i = 0
    const len = pattern.length

    // Characters that end the literal prefix
    const metaChars = new Set(['.', '(', '[', '|', '?', '*', '+', '{'])

    while (i < len) {
        const char = pattern[i]

        // Escaped character — skip both the backslash and the next char
        if (char === '\\') {
            i += 2
            continue
        }

        if (metaChars.has(char)) {
            break
        }

        i++
    }

    // If i is 0, the pattern starts with a meta-character → full scan required
    return i === 0
}