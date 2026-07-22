import type { DictSettingInfo, SessionConfig } from '@/common/type-interface';
import { useSystemConfigStore } from '@/stores/stores'
const systemConfigStore = useSystemConfigStore();



const getDictSettingsForLookup = (dict_setting_option_name: string) => {
    // return a list that contains the dict name which is not disabled in the same order.
    let dictnames: string[] = []
    systemConfigStore.systemConfig.dict_set_options[dict_setting_option_name].filter((item: DictSettingInfo) => item.is_enabled).map(item => dictnames.push(item.name))
    return dictnames
}

const getDefaultSessionConfig = (sessionName: string) => {

    let sessionConfig: SessionConfig = {
        name: sessionName,
        default_folder: { "id": null },
        dict_setting_option_name: "default",
        default_search_method: { "method": "prefix_search" },
        pin: { "is_pinned": true }
    }
    return sessionConfig
}

/**
 * FST词典自动机专用：判断正则是否会遍历全部词条节点
 * 特性：引擎强制从字符串头部匹配（隐式^），不再校验开头是否带^
 * @param pattern 用户正则原始字符串
 * @returns true: 需要全量遍历; false: 可提取前缀、FST路径剪枝
 */
function willScanAllFstNodes(pattern: string): boolean {
    if (!pattern) return true;

    let i = 0;
    const len = pattern.length;

    // 遇到这些字符代表【固定字面前缀终止】
    const stopSymbols = new Set(['.', '(', '[', '|', '?', '*', '+', '{']);

    while (i < len) {
        const char = pattern[i];

        // 处理转义字符 \，跳过下一个字符
        if (char === '\\') {
            i += 2;
            continue;
        }

        // 碰到不定匹配元字符，前缀到此结束
        if (stopSymbols.has(char)) {
            break;
        }

        // 普通字面字符，继续延长前缀
        i++;
    }

    // i === 0：最起始位置直接就是不定匹配符号，无任何可用字面前缀 → 全量扫描
    return i === 0;
}


export { getDictSettingsForLookup, getDefaultSessionConfig, willScanAllFstNodes }