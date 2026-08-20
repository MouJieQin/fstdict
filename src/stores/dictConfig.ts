import { defineStore } from 'pinia'
import type { DictsSettingInfo } from '@/common/type-interface'

interface DictConfigState {
    dictConfig: {
        dict_set_options: Record<string, DictsSettingInfo>
    } | null
}

export const useDictConfigStore = defineStore('dictConfig', {
    state: (): DictConfigState => ({
        dictConfig: null,
    }),
    actions: {
        setDictConfig(config: DictConfigState['dictConfig']) {
            this.dictConfig = config
        },
    },
})