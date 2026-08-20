// ============================================================
// Core type definitions
// ============================================================

// --- Dictionary metadata ---
export interface DictInfo {
    name: string
    path: string
    root: string
    css: string[]
    js: string[]
    data: string
    cover: string
    cover_url: string
}

export type DictsInfo = Record<string, DictInfo>

// --- Dictionary set options ---
export interface DictSettingInfo {
    name: string
    is_enabled: boolean
}

export type DictsSettingInfo = DictSettingInfo[]

export interface DictSetOptions {
    [optionName: string]: DictsSettingInfo
}

export interface DictConfig {
    dict_set_options: DictSetOptions
}

// --- Session configuration ---
export interface SessionDefaultFolder {
    id: number | null
}

export interface SessionDefaultSearchMethod {
    method: string
}

export interface SessionPin {
    is_pinned: boolean
}

export interface SessionNameId {
    id: number
    name: string
}

export interface SessionConfig {
    name: string
    dict_setting_option_name: string
    default_folder: SessionDefaultFolder
    default_search_method: SessionDefaultSearchMethod
    ocr_lang_type: string
    pin?: SessionPin
}

// --- Folder configuration ---
export interface FolderInfo {
    id: number
    name: string
    description: string
    words_count: number
    created_at: string
}

export interface FolderConfig {
    folders: {
        folder_info: FolderInfo[]
    }
}

// --- Word records ---
export interface WordInfo {
    word: string
    created_at: string | null
    query_count: number
}

export interface WordInfoWithFavoriteAt extends WordInfo {
    favorited_at: string | null
}

export interface FolderWords {
    [folder_id: number]: WordInfoWithFavoriteAt[]
}

export interface WordInfoWithLastSearch extends WordInfo {
    last_searched: string | null
}

// --- System configuration ---
export interface AppearanceConfig {
    theme: 'light' | 'dark' | 'auto'
}

export interface OcrConfig {
    lang_types: Record<string, string>
    session: { id: number }
}

export interface AppSession {
    id: number
}

export interface HelperSelectionConfig {
    session: { id: number }
}

export interface AppConfig {
    session: AppSession
    helper_selection: HelperSelectionConfig
}

export interface SystemConfig {
    appearance: AppearanceConfig
    ocr: OcrConfig
    app: AppConfig
}

// --- Chat / message (kept for compatibility) ---
export interface Message {
    message_id: number
    raw_text: string
    secondary_response: string | null
    processed_html: string
    time: string
    role: 'user' | 'assistant' | 'system'
    is_playing: boolean
}

// --- Lookup result ---
export interface LookupResult {
    keyword: string
    note: string
    left_history: boolean
    result: Record<string, string[]> | null
    is_word_favorited: boolean
}

// --- Anki progress data ---
export interface AnkiProgressData {
    total_count: number
    count: number
    updated_count: number
    created_count: number
    update_error_count: number
    create_error_count: number
    error_message?: string
}

export interface AnkiProgressMessage {
    type: string
    deck_name: string
    data: AnkiProgressData
}

// --- Add dictionary message ---
export interface AddDictMessage {
    type: 'info' | 'warning' | 'error' | 'success' | 'done'
    msg: string
}

// --- Iframe cross-window messages ---
export interface IframeBaseMessage {
    type: string
    iframeId: string
}

export interface IframeEntryMessage extends IframeBaseMessage {
    type: 'ENTRY_CLICK'
    entry: string
}

export interface IframeSoundMessage extends IframeBaseMessage {
    type: 'SOUND_CLICK'
    sound: string
}

export interface IframeLocationMessage extends IframeBaseMessage {
    type: 'LOCATION_CLICK'
    elementOffsetTop: number
}

export interface IframeKeydownMessage extends IframeBaseMessage {
    type: 'KEYDOWN'
    key: string
    code: string
    ctrlKey: boolean
    shiftKey: boolean
    altKey: boolean
    metaKey: boolean
}

export type IframeMessage =
    | IframeEntryMessage
    | IframeSoundMessage
    | IframeLocationMessage
    | IframeKeydownMessage