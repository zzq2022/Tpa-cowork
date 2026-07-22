import "@testing-library/jest-dom/vitest"

import i18n, { i18nReady } from "@/i18n/i18n"

// Host OS may be zh-CN; pin unit tests to English after the startup locale
// promise settles so slash-command display helpers stay deterministic.
await i18nReady
await i18n.changeLanguage("en")
