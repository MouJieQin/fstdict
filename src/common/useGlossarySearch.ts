import { computed, ref, type Ref } from 'vue'

/**
 * Glossary word filtering with pluggable search methods.
 *
 * Extracted from GlossaryOptions.vue so the sort/filter pipeline is testable
 * and reusable by every consumer (DictPage side panel, SessionGlossary
 * upper/lower panels) without duplicating the scoring logic.
 *
 * Search methods:
 *  - FUZZY   : subsequence match with scoring - typed "fa" matches both
 *              "favor" and "format" (f-a appear in order), and prefix
 *              matches are ranked first, so "favor" comes before "format".
 *              This is the same "type-ahead / quick-open" style used by
 *              VS Code and IntelliJ (a lightweight version of fuzzy
 *              matching: no typo tolerance, no per-char weights).
 *  - CONTAINS: plain case-insensitive substring match, keeps source order.
 *  - REGEX   : the query is compiled as a RegExp (case-insensitive); an
 *              invalid pattern yields an empty result set instead of
 *              crashing the pipeline.
 */

export const GLOSSARY_SEARCH_METHOD = {
    FUZZY: 'fuzzy',
    CONTAINS: 'contains',
    REGEX: 'regex',
} as const

export type GlossarySearchMethod =
    (typeof GLOSSARY_SEARCH_METHOD)[keyof typeof GLOSSARY_SEARCH_METHOD]

/** Scoring for the fuzzy method. Lower score = better rank. -1 = no match. */
function fuzzyScore(query: string, word: string): number {
    const q = query.toLowerCase()
    const w = word.toLowerCase()
    if (!q) return 0

    let qi = 0
    let firstMatch = -1
    let score = 0
    for (let wi = 0; wi < w.length && qi < q.length; wi++) {
        if (w[wi] === q[qi]) {
            if (firstMatch === -1) firstMatch = wi
            // Later characters cost more: matches near the word start rank higher.
            score += wi + 1
            qi++
        }
    }
    if (qi < q.length) return -1 // not a subsequence -> no match

    // Strongest signal: the word starts with the query (prefix match).
    if (w.startsWith(q)) score -= 10000
    // Second signal: how early the first matched character appears.
    score -= firstMatch * 100
    // Tie-breaker: shorter words win.
    score += Math.min(w.length, 100)
    return score
}

/**
 * Reactive search state bound to a word list.
 *
 * @param words source list (already sorted by the active sort method)
 */
export function useGlossarySearch(words: Ref<string[]>) {
    const query = ref('')
    const method = ref<GlossarySearchMethod>(GLOSSARY_SEARCH_METHOD.FUZZY)

    const filteredWords = computed<string[]>(() => {
        const q = query.value.trim()
        if (!q) return words.value

        switch (method.value) {
            case GLOSSARY_SEARCH_METHOD.CONTAINS: {
                const needle = q.toLowerCase()
                return words.value.filter((w) => w.toLowerCase().includes(needle))
            }
            case GLOSSARY_SEARCH_METHOD.REGEX: {
                let re: RegExp
                try {
                    re = new RegExp(q, 'i')
                } catch {
                    return [] // invalid pattern: nothing matches
                }
                return words.value.filter((w) => re.test(w))
            }
            case GLOSSARY_SEARCH_METHOD.FUZZY:
            default: {
                const scored = words.value
                    .map((word) => ({ word, score: fuzzyScore(q, word) }))
                    // score === -1 means "no subsequence match"; any other
                    // value (positive OR negative) is a match - prefix
                    // matches legitimately score negative due to their bonus.
                    .filter((item) => item.score !== -1)
                // Re-rank by relevance; prefix matches naturally float to top.
                scored.sort((a, b) => a.score - b.score)
                return scored.map((item) => item.word)
            }
        }
    })

    const clearSearch = (): void => {
        query.value = ''
    }

    return { query, method, filteredWords, clearSearch }
}
