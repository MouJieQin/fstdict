import { createI18n } from 'vue-i18n'
import { ref } from 'vue'
import en from '@/locales/en.json'
import zh from '@/locales/zh.json'

// Typed Element Plus locale imports (official recommended path)
import enLocale from 'element-plus/es/locale/lang/en'
import zhCnLocale from 'element-plus/es/locale/lang/zh-cn'
import type { Language } from 'element-plus/es/locale'

export type AppLocale = 'en' | 'zh'

export const elementPlusLocales: Record<AppLocale, Language> = {
    en: enLocale,
    zh: zhCnLocale,
}

export const i18n = createI18n({
    legacy: false,
    locale: 'en',
    fallbackLocale: 'en',
    messages: { en, zh }
})

/**
 * Reactive Element Plus locale for ElConfigProvider
 */
export const elementPlusLocale = ref<Language>(enLocale)

/**
 * Switch language for both vue-i18n and Element Plus
 */
export function setAppLocale(lang: string): void {
    const valid: AppLocale = (lang === 'en' || lang === 'zh') ? lang : 'en'

    if (i18n.global.locale.value !== valid) {
        i18n.global.locale.value = valid
        elementPlusLocale.value = elementPlusLocales[valid]
    }
}

export default i18n