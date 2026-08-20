import { WebSocketService } from '@/common/websocket-client'
import type { SessionConfig } from '@/common/type-interface'
import { WS_BASE_URL } from '@/common/constants'

export class SessionWebSocketService extends WebSocketService {
    constructor(sessionId: number) {
        super(`${WS_BASE_URL}${sessionId}`)
    }

    private sendTyped(type: string, data: Record<string, unknown> = {}): void {
        this.send({ type, data })
    }

    // --- Note operations ---
    public sendSaveWordNote(keyword: string, note: string): void {
        this.sendTyped('save_word_note', { keyword, note })
    }

    public sendDeleteWordNote(keyword: string): void {
        this.sendTyped('delete_word_note', { keyword })
    }

    // --- Lookup operations ---
    public sendLookupKeyword(
        keyword: string,
        folderId: number | null,
        dictSettings: string[] | null = null,
        leftHistory = true
    ): void {
        this.sendTyped('lookup_keyword', {
            keyword,
            folder_id: folderId,
            dict_settings: dictSettings,
            left_history: leftHistory,
        })
    }

    public sendLookupKeywordRequest(keyword: string): void {
        this.sendTyped('lookup_keyword_request', { keyword })
    }

    // --- Configuration ---
    public sendFolderConfig(): void {
        this.sendTyped('folder_config')
    }

    public sendUpdateDictConfig(dictConfig: unknown): void {
        this.sendTyped('update_dict_config', { dict_config: dictConfig })
    }

    public sendUpdateSystemConfig(systemConfig: unknown): void {
        this.sendTyped('update_system_config', { system_config: systemConfig })
    }

    // --- Favorites ---
    public sendToggleFavor(keyword: string, folderId: number | null): void {
        this.sendTyped('toggle_favor', { keyword, folder_id: folderId })
    }

    // --- Folder management ---
    public sendCreateFolder(name: string, description: string): void {
        this.sendTyped('create_folder', { folder_name: name, folder_description: description })
    }

    public sendDeleteFolder(folderId: number): void {
        this.sendTyped('delete_folder', { folder_id: folderId })
    }

    public sendUpdateFolder(folderId: number, name: string, description: string): void {
        this.sendTyped('update_folder', {
            folder_id: folderId,
            folder_name: name,
            folder_description: description,
        })
    }

    // --- Dictionary set options ---
    public sendCreateDictSetOption(optionName: string): void {
        this.sendTyped('create_dict_set_option', { option_name: optionName })
    }

    public sendRemoveDictSetOption(optionName: string): void {
        this.sendTyped('remove_dict_set_option', { option_name: optionName })
    }

    public sendRenameDictSetOption(oldName: string, newName: string): void {
        this.sendTyped('rename_dict_set_option', {
            old_option_name: oldName,
            new_option_name: newName,
        })
    }

    // --- Session management ---
    public sendSessionConfig(config: SessionConfig): void {
        this.sendTyped('session_config', { config })
    }

    public sendCreateSession(config: SessionConfig): void {
        this.sendTyped('create_session', { config })
    }

    public sendRemoveSession(): void {
        this.sendTyped('remove_session')
    }

    public sendRenameSession(name: string): void {
        this.sendTyped('rename_session', { name })
    }

    // --- Data requests ---
    public sendFavoriteWordsRequest(folderId: number): void {
        this.sendTyped('favorite_words_request', { folder_id: folderId })
    }

    public sendSearchHistoryRequest(): void {
        this.sendTyped('search_history_request')
    }

    // --- Floating window ---
    public sendFloatingWindowPinClick(sessionId: number, isPinned: boolean): void {
        this.sendTyped('toggle_floating_pin', {
            session_id: sessionId,
            is_pinned: isPinned,
        })
    }

    public sendNoteIsEditing(isEditing: boolean): void {
        this.sendTyped('note_is_editing', { is_editing: isEditing })
    }

    // --- Dictionary management ---
    public sendAddDictionary(dictPath: string): void {
        this.sendTyped('add_dictionary', { dict_path: dictPath })
    }

    public sendShowDictInFolder(dictName: string): void {
        this.sendTyped('show_dict_in_folder', { dict_name: dictName })
    }

    public sendDeleteDict(dictName: string): void {
        this.sendTyped('delete_dictionary', { dict_name: dictName })
    }

    // --- Autocomplete ---
    public sendKeywordOptionsSearch(
        keyword: string,
        searchMethod = 'prefix_search',
        dictSettings: string[] | null = null
    ): void {
        this.sendTyped('keyword_options_search', {
            keyword,
            search_method: searchMethod,
            dict_settings: dictSettings,
        })
    }

    public sendKeywordOptionsNote(keyword: string, note: string): void {
        this.sendTyped('word_option_note', { keyword, options: [note] })
    }

    // --- Anki ---
    public sendUpdateToAnki(deckName: string, folderId: number): void {
        this.sendTyped('update_to_anki', { folder_id: folderId, deck_name: deckName })
    }

    public sendCancelAnkiUpdate(): void {
        this.sendTyped('cancel_anki_update')
    }

    // --- Legacy alias ---
    public sendToggleFloatingWindowPin(fullPath: string): void {
        this.sendTyped('toggle_float_pin', { full_path: fullPath })
    }
}

export function useSessionWebSocket(id: number): SessionWebSocketService {
    return new SessionWebSocketService(id)
}