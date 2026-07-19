
export interface DictInfo {
    name: string;
    path: string;
    root: string;
    css: string[];
    js: string[];
    data: string;
    cover: string;
}

export interface DictsInfo {
    [dict_name: string]: DictInfo;
}

export interface DictSettingInfo {
    name: string;
    cover_url: string;
    is_enabled: boolean;
}

export type DictsSettingInfo = DictSettingInfo[]

export interface SessiondefaultFolder {
    id: number | null;
    // name: string;
}

export interface SessionDefaultSearchMethod {
    method: string;
}

export interface SessionPin {
    is_pinned: boolean;
}


export interface SessionConfig {
    dictsSettingInfoName: string;
    default_folder: SessiondefaultFolder;
    default_search_method: SessionDefaultSearchMethod;
    pin?: SessionPin;
}

export interface FolderInfo {
    id: number;
    name: string;
    description: string;
    words_count: number;
    created_at: string;
}

export interface FolderConfig {
    folders: {
        folder_info: FolderInfo[];
    }
}

export interface WordInfo {
    word: string;
    created_at: string | null;
    query_count: number;
}

export interface WordInfoWithFavoriteAt extends WordInfo {
    favorited_at: string | null;
}

export interface FolderWords {
    [folder_id: number]: WordInfoWithFavoriteAt[];
}

export interface WordInfoWithLastSearch extends WordInfo {
    last_searched: string | null;
}

export interface Message {
    message_id: number;
    raw_text: string;
    secondary_response: string | null;
    processed_html: string;
    time: string;
    role: 'user' | 'assistant' | 'system';
    is_playing: boolean;
}

