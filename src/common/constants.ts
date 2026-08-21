// ============================================================
// Application-wide constants
// ============================================================

// --- WebSocket ---
export const WS_BASE_URL = 'ws://127.0.0.1:5959/ws/dictionary/session/'
export const API_BASE_URL = 'http://127.0.0.1:5959/api/download?path='

// --- Reconnection ---
export const WS_RECONNECT_BASE_DELAY = 500
export const WS_RECONNECT_MAX_DELAY = 5000
export const WS_RECONNECT_THRESHOLD = 10

// --- Search / Autocomplete ---
export const DEBOUNCE_SEARCH_MS = 300
export const AUTOCOMPLETE_ITEM_HEIGHT = 35
export const AUTOCOMPLETE_OVERSCAN = 10
export const AUTOCOMPLETE_PANEL_HEIGHT = 250

export const WORD_OPTIONS_ITEM_HEIGHT = 30
export const WORD_OPTIONS_OVERSCAN = 20

// --- FST prefix detection ---
export const REGEX_META_CHARS = ['.', '(', '[', '|', '?', '*', '+', '{'] as const

// --- Status prefixes for word option messages ---
export const OPTION_PREFIX = {
    ERROR: 'FSTD_ERROR',
    WARN: 'FSTD_WARN',
    SEARCHING: 'FSTD_SEARCHING',
} as const

// --- Validation limits ---
export const MAX_SESSION_NAME_LENGTH = 30
export const MAX_FOLDER_NAME_LENGTH = 20
export const MAX_DICT_SET_OPTION_NAME_LENGTH = 30

// --- Iframe ---
export const IFRAME_HEIGHT_PADDING = 10
export const IFRAME_HEIGHT_DEBOUNCE_MS = 200

// --- Layout ---
export const HEADER_HEIGHT_VAR = '--header-height'
export const MOBILE_BREAKPOINT = 700
export const WORD_OPTIONS_DEFAULT_WIDTH = 300
export const MIN_DETAIL_PANEL_WIDTH = 400

// --- Default values ---
export const DEFAULT_SEARCH_METHOD = 'prefix_search'
export const DEFAULT_DICT_SET_OPTION = 'default'
export const DEFAULT_OCR_LANG = 'English'
export const DEFAULT_THEME = 'light'

// --- Search method identifiers ---
export const SEARCH_METHOD = {
    PREFIX: 'prefix_search',
    REGEX: 'regex_search',
    PREFIX_DISTANCE: 'prefix_distance_search',
    FUZZY: 'suggest_search',
} as const

// --- Environment identifiers ---
export const ENV = {
    MAIN: '',
    HELPER: 'helper_main_tauri',
    SELECTION: 'selection_float_search',
    IWIN: 'iwin',
    ANKI: 'anki',
    FLOATING: 'floating_tauri',
} as const

// --- Tauri command names ---
export const TAURI_CMD = {
    SET_SELECTION_WINDOW_PINNED: 'set_selection_window_pinned',
    SET_MAIN_WINDOW_PINNED: 'set_main_window_pinned',
    SET_THEME: 'set_theme',
    CHECK_ACCESSIBILITY: 'check_accessibility',
    REQUEST_ACCESSIBILITY: 'request_accessibility',
    LAUNCH_CGEVENT_SERVER: 'launch_cgevent_server',
    LAUNCH_HELPER: 'launch_helper',
    TRIGGER_NOTIFICATION: 'trigger_notification',
} as const

// --- Tauri event names ---
export const TAURI_EVENT = {
    TEXT_SELECTED: 'cgevent-select',
    OCR_RESULT: 'cgevent-ocr',
} as const

// --- WebSocket message types ---
export const WS_MSG = {
    DICT_INFO: 'dict_info',
    KEYWORD_OPTIONS_SEARCH: 'keyword_options_search',
    LOOKUP_KEYWORD_REQUEST: 'lookup_keyword_request',
    WORD_NOTE: 'word_note',
    LOOKUP_KEYWORD: 'lookup_keyword',
    CREATE_SESSION: 'create_session',
    SESSION_CONFIG: 'session_config',
    SESSIONS_NAME_ID: 'sessions_name_id',
    TOGGLE_FLOATING_PIN: 'toggle_floating_pin',
    TOGGLE_FAVOR: 'toggle_favor',
    FAVORITE_WORDS: 'favorite_words',
    SEARCH_HISTORY: 'search_history',
    FOLDER_CONFIG: 'folder_config',
    DICT_CONFIG: 'dict_config',
    SYSTEM_CONFIG: 'system_config',
    CLOSE_FIXED_WINDOW: 'close_fixed_window',
    ANKI_PROGRESS: 'anki_progress',
    ADD_DICTIONARY: 'add_dictionary',
    CGEVENT: 'cgevent',
    TAURI_NOTIFICATION: 'tauri_notification',
    ERROR_SESSION_NOT_EXIST: 'error_session_not_exist',
} as const

// --- Iframe message types ---
export const IFRAME_MSG = {
    ENTRY_CLICK: 'ENTRY_CLICK',
    SOUND_CLICK: 'SOUND_CLICK',
    LOCATION_CLICK: 'LOCATION_CLICK',
    KEYDOWN: 'KEYDOWN',
} as const

// --- Iframe URL schemes ---
export const URL_SCHEME = {
    ENTRY: 'entry://',
    SOUND: 'sound://',
    FILE: 'file:/',
} as const

// --- Anki progress states ---
export const ANKI_STATE = {
    ACQUIRING: 'trying_acquiring_cards_from_anki',
    PROGRESS: 'progress',
    DONE: 'done',
    ERROR: 'error',
    CANCELED: 'canceled',
} as const

// --- Add dictionary message types ---
export const ADD_DICT_MSG = {
    INFO: 'info',
    WARNING: 'warning',
    ERROR: 'error',
    SUCCESS: 'success',
    DONE: 'done',
} as const