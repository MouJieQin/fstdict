import { createI18n } from 'vue-i18n'
import { ref } from 'vue'
import en from '@/locales/en.json'
import zh from '@/locales/zh.json'
import ja from '@/locales/ja.json'   // new
import ko from '@/locales/ko.json'   // new

// Element Plus typed locale imports
import enLocale from 'element-plus/es/locale/lang/en'
import zhCnLocale from 'element-plus/es/locale/lang/zh-cn'
import jaLocale from 'element-plus/es/locale/lang/ja'   // new
import koLocale from 'element-plus/es/locale/lang/ko'   // new
import type { Language } from 'element-plus/es/locale'

export type AppLocale = 'en' | 'zh' | 'ja' | 'ko'  // extended

export const elementPlusLocales: Record<AppLocale, Language> = {
    en: enLocale,
    zh: zhCnLocale,
    ja: jaLocale,   // new
    ko: koLocale,   // new
}

export const i18n = createI18n({
    legacy: false,
    locale: 'en',
    fallbackLocale: 'en',
    messages: { en, zh, ja, ko }  // extended
})

export const elementPlusLocale = ref<Language>(enLocale)

export function setAppLocale(lang: string): void {
    // Extended validation
    const valid: AppLocale =
        (lang === 'en' || lang === 'zh' || lang === 'ja' || lang === 'ko')
            ? lang
            : 'en'

    if (i18n.global.locale.value !== valid) {
        i18n.global.locale.value = valid
        elementPlusLocale.value = elementPlusLocales[valid]
    }
}

export default i18n