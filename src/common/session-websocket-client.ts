import { WebSocketService } from '@/common/websocket-client'
import type { SessionConfig, SessiondefaultFolder } from '@/common/type-interface'

class SessionWebSocketService extends WebSocketService {
    constructor(url: string) {
        super(url)
    }

    private _send(type: string, data: {} | null = null) {
        this.send({
            type: type,
            data: {
                ...data,
            },
        })
    }

    sendSaveWordNote(keyword: string, note: string) {
        this._send(
            'save_word_note',
            {
                keyword: keyword,
                note: note,
            }
        )
    }

    sendDeleteWordNote(keyword: string) {
        this._send(
            'delete_word_note',
            {
                keyword: keyword,
            }
        )
    }

    // 发送用户输入
    sendLookupKeyword(keyword: string, folder_id: number | null, dictSettings: string[] | null = null, leftHistory: boolean = true) {
        this._send(
            'lookup_keyword',
            {
                keyword: keyword,
                folder_id: folder_id,
                dict_settings: dictSettings,
                left_history: leftHistory,
            }
        )
    }

    sendFolderConfig() {
        this._send(
            'folder_config',
            {}
        )
    }

    sendUpdateSystemConfig(systemConfig: any) {
        this._send(
            'update_system_config',
            {
                system_config: systemConfig
            }
        )
    }

    sendLookupKeywordRequest(keyword: string) {
        this._send(
            'lookup_keyword_request',
            {
                keyword: keyword,
            }
        )
    }

    sendToggleFavor(keyword: string, folder_id: number | null) {
        this._send(
            'toggle_favor',
            {
                keyword: keyword,
                folder_id: folder_id,
            }
        )
    }

    sendCreateFolder(folderName: string, folderDescription: string) {
        this._send(
            'create_folder',
            {
                folder_name: folderName,
                folder_description: folderDescription,
            }
        )
    }

    sendCreateDictSetOption(optionName: string) {
        this._send(
            'create_dict_set_option',
            {
                option_name: optionName
            }
        )
    }

    sendDeleteFolder(folderId: number) {
        this._send(
            'delete_folder',
            {
                folder_id: folderId,
            }
        )
    }

    sendUpdateToAnki(deck_name: string, folderId: number) {
        this._send(
            'update_to_anki',
            {
                folder_id: folderId,
                deck_name: deck_name,
            }
        )
    }

    sendCancelAnkiUpdate() {
        this._send(
            'cancel_anki_update',
            {}
        )
    }


    sendUpdateFolder(folderId: number, folderName: string, folderDescription: string) {
        this._send(
            'update_folder',
            {
                folder_id: folderId,
                folder_name: folderName,
                folder_description: folderDescription,
            }
        )
    }

    sendSessionConfig(config: SessionConfig) {
        this._send(
            'session_config',
            {
                config: config,
            }
        )
    }

    sendFavoriteWordsRequest(folderId: number) {
        this._send(
            'favorite_words_request',
            {
                folder_id: folderId,
            }
        )
    }

    sendSearchHistoryRequest() {
        this._send(
            'search_history_request',
            {}
        )
    }

    sendFloatingWindowPinClick(sessionId: number, is_pinned: boolean) {
        this._send(
            'toggle_floating_pin',
            {
                session_id: sessionId,
                is_pinned: is_pinned,
            }
        )
    }

    sendNoteIsEditing(isEditing: boolean) {
        this._send(
            'note_is_editing',
            {
                is_editing: isEditing,
            }
        )
    }

    sendAddDictionary(dictPath: string) {
        this._send(
            'add_dictionary',
            {
                dict_path: dictPath, // 使用传入的字典设置
            }
        )
    }

    sendShowDictInFolder(dictName: string) {
        this._send(
            'show_dict_in_folder',
            {
                dict_name: dictName,
            }
        )
    }

    sendDeleteDict(dictName: string) {
        this._send(
            'delete_dictionary',
            {
                dict_name: dictName,
            }
        )
    }



    sendKeywordOptionsSearch(keyword: string, searchMethod: string = 'prefix_search', dictSettings: string[] | null = null) {
        this._send(
            'keyword_options_search',
            {
                keyword: keyword,
                search_method: searchMethod, // 使用传入的搜索方法
                dict_settings: dictSettings, // 使用传入的字典设置
            }
        )
    }


    sendToggleFloatingWindowPin(fullPath: string) {
        this._send('toggle_float_pin', {
            full_path: fullPath,
        })
    }


}

// 导出单例或工厂函数，根据项目需求选择
let sessionWebSocketInstance: SessionWebSocketService | null = null;
export function useSessionWebSocket(id: number) {
    sessionWebSocketInstance = new SessionWebSocketService("ws://localhost:5959/ws/dictionary/session/" + id);
    return sessionWebSocketInstance;
}
export { SessionWebSocketService }
