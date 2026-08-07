# Expanded Icon Sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the 76px icon-only app rail into a 144px labeled navigation rail that defaults to expanded, can be manually collapsed, remembers the user's choice, and temporarily collapses below 960px.

**Architecture:** Keep the preference entirely client-side because it is presentation-only state and is not an `AppConfig`/`UserConfig` setting. A focused preference module owns normalization and localStorage access; `IconSidebar` combines that saved preference with the existing viewport media-query hook to derive the effective state. A small internal navigation-row component centralizes label, tooltip, active state, badge, and expanded/collapsed rendering without introducing a global navigation framework.

**Tech Stack:** React 19, TypeScript, Tailwind CSS, shadcn/Radix Tooltip through `IconTip`, Vitest, Testing Library, i18next.

## Global Constraints

- Expanded width is exactly 144px; collapsed width remains exactly 76px.
- The default saved preference is expanded.
- Responsive collapse applies at `max-width: 959px` and never writes to localStorage.
- Expanded rows show one-line labels only; collapsed rows show `name｜short purpose` in `IconTip`.
- Hover and selected states may only deepen the background; do not add hover borders, rings, shadows, or scaling.
- Use Tailwind utility classes only; do not add inline styles or custom CSS.
- Reuse `@/components/ui/tooltip` through `IconTip`; do not use native `title`.
- Preserve unread badges, context menus, server status, update status, language menu, theme switcher, help, profile, settings, and about behavior.
- Add every new i18n key to `zh.json`, `zh-TW.json`, and `en.json` in the same change.
- Do not add an `AppConfig` or `UserConfig` field for this UI-only preference.
- Before running any test command during implementation, ask the user as required by the repository pre-push policy.
- Do not commit or push unless the user explicitly requests it.

---

### Task 1: Sidebar display preference

**Files:**
- Create: `src/components/common/iconSidebarPreference.ts`
- Create: `src/components/common/iconSidebarPreference.test.ts`
- Modify: `src/components/settings/SettingsResetControl.tsx`
- Modify: `src/components/settings/SettingsResetControl.test.tsx`

**Interfaces:**
- Produces: `IconSidebarDisplayMode = "expanded" | "collapsed"`.
- Produces: `ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY = "hope.iconSidebarDisplayMode"`.
- Produces: `readIconSidebarDisplayModePreference(fallback?): IconSidebarDisplayMode`.
- Produces: `writeIconSidebarDisplayModePreference(mode): void`.
- Produces: `resetIconSidebarDisplayModePreference(): IconSidebarDisplayMode`.

- [ ] **Step 1: Write failing preference tests**

```ts
import { beforeEach, describe, expect, it } from "vitest"
import {
  ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY,
  readIconSidebarDisplayModePreference,
  resetIconSidebarDisplayModePreference,
  writeIconSidebarDisplayModePreference,
} from "./iconSidebarPreference"

describe("iconSidebarPreference", () => {
  beforeEach(() => window.localStorage.clear())

  it("defaults to expanded when no valid value is stored", () => {
    expect(readIconSidebarDisplayModePreference()).toBe("expanded")
    window.localStorage.setItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY, "invalid")
    expect(readIconSidebarDisplayModePreference()).toBe("expanded")
  })

  it("round-trips a collapsed preference", () => {
    writeIconSidebarDisplayModePreference("collapsed")
    expect(readIconSidebarDisplayModePreference()).toBe("collapsed")
  })

  it("resets to expanded", () => {
    writeIconSidebarDisplayModePreference("collapsed")
    expect(resetIconSidebarDisplayModePreference()).toBe("expanded")
    expect(window.localStorage.getItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY)).toBeNull()
  })
})
```

- [ ] **Step 2: After user approval, run the focused test and verify failure**

Run: `pnpm vitest run src/components/common/iconSidebarPreference.test.ts`

Expected: FAIL because `iconSidebarPreference.ts` does not exist.

- [ ] **Step 3: Implement the preference module**

```ts
export type IconSidebarDisplayMode = "expanded" | "collapsed"

export const ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY = "hope.iconSidebarDisplayMode"

export function normalizeIconSidebarDisplayMode(
  value: unknown,
): IconSidebarDisplayMode | null {
  return value === "expanded" || value === "collapsed" ? value : null
}

export function readIconSidebarDisplayModePreference(
  fallback: IconSidebarDisplayMode = "expanded",
): IconSidebarDisplayMode {
  if (typeof window === "undefined") return fallback
  try {
    return (
      normalizeIconSidebarDisplayMode(
        window.localStorage.getItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY),
      ) ?? fallback
    )
  } catch {
    return fallback
  }
}

export function writeIconSidebarDisplayModePreference(
  mode: IconSidebarDisplayMode,
): void {
  if (typeof window === "undefined") return
  window.localStorage.setItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY, mode)
}

export function resetIconSidebarDisplayModePreference(): IconSidebarDisplayMode {
  if (typeof window === "undefined") return "expanded"
  try {
    window.localStorage.removeItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY)
  } catch {
    // Restricted webviews may not expose writable localStorage.
  }
  return "expanded"
}
```

- [ ] **Step 4: Make reset-all remove the new preference**

Import `resetIconSidebarDisplayModePreference` in `SettingsResetControl.tsx` and call it in the existing client-side preference reset block next to the chat display-mode reset. Extend `SettingsResetControl.test.tsx` by storing `"collapsed"`, triggering reset, and asserting the storage key is absent.

```ts
window.localStorage.setItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY, "collapsed")
// trigger the existing reset-all action
expect(window.localStorage.getItem(ICON_SIDEBAR_DISPLAY_MODE_STORAGE_KEY)).toBeNull()
```

- [ ] **Step 5: After user approval, run focused tests**

Run: `pnpm vitest run src/components/common/iconSidebarPreference.test.ts src/components/settings/SettingsResetControl.test.tsx`

Expected: both files PASS.

---

### Task 2: Labeled navigation rail and responsive effective state

**Files:**
- Create: `src/components/common/IconSidebarNavItem.tsx`
- Create: `src/components/common/IconSidebar.test.tsx`
- Modify: `src/components/common/IconSidebar.tsx`

**Interfaces:**
- Consumes: `IconSidebarDisplayMode`, `readIconSidebarDisplayModePreference`, and `writeIconSidebarDisplayModePreference` from Task 1.
- Consumes: `useViewportMediaQuery("(max-width: 959px)")`.
- Produces: `IconSidebarNavItem` with `icon`, `label`, `tooltip`, `expanded`, `active`, `onClick`, `badge`, and optional `className` props.

- [ ] **Step 1: Write failing component tests**

Mock `useViewportMediaQuery`, `IconTip`, transport, unread stores, theme, and manual-window helpers using the same Vitest patterns already used under `src/components`. Cover these observable contracts:

```tsx
it("renders labels and a 144px rail by default", () => {
  render(<IconSidebar {...requiredProps} />)
  expect(screen.getByText("会话")).toBeVisible()
  expect(screen.getByRole("navigation")).toHaveClass("w-36")
})

it("manually collapses and persists the choice", async () => {
  const user = userEvent.setup()
  render(<IconSidebar {...requiredProps} />)
  await user.click(screen.getByRole("button", { name: "收起导航" }))
  expect(screen.getByRole("navigation")).toHaveClass("w-[76px]")
  expect(readIconSidebarDisplayModePreference()).toBe("collapsed")
})

it("temporarily collapses on a narrow viewport without overwriting preference", () => {
  vi.mocked(useViewportMediaQuery).mockReturnValue(true)
  writeIconSidebarDisplayModePreference("expanded")
  render(<IconSidebar {...requiredProps} />)
  expect(screen.getByRole("navigation")).toHaveClass("w-[76px]")
  expect(readIconSidebarDisplayModePreference()).toBe("expanded")
})

it("keeps unread badges and context menu actions available when collapsed", async () => {
  writeIconSidebarDisplayModePreference("collapsed")
  render(<IconSidebar {...requiredProps} totalUnreadCount={3} />)
  expect(screen.getByText("3")).toBeInTheDocument()
  expect(screen.getByLabelText(/会话/)).toBeEnabled()
})
```

- [ ] **Step 2: After user approval, run the component test and verify failure**

Run: `pnpm vitest run src/components/common/IconSidebar.test.tsx`

Expected: FAIL because the rail has no navigation role, expanded labels, or toggle.

- [ ] **Step 3: Implement the shared row component**

`IconSidebarNavItem` must render one full-width row in expanded mode and a centered icon button in collapsed mode. Keep `IconTip` mounted in both modes; use `label` alone when expanded and `tooltip` when collapsed. The active/hover class contract is:

```tsx
className={cn(
  "h-8 rounded-xl transition-colors duration-200",
  expanded ? "w-full justify-start gap-2.5 px-3" : "w-8 justify-center px-0",
  active
    ? "bg-indigo-500/15 text-indigo-600 dark:text-indigo-400"
    : "text-muted-foreground hover:bg-indigo-500/10 hover:text-indigo-600 dark:hover:text-indigo-400",
  className,
)}
```

Render the visible label only when `expanded`; wrap it with `min-w-0 truncate text-xs font-medium`, and expose `aria-current={active ? "page" : undefined}`.

- [ ] **Step 4: Add saved and responsive state to `IconSidebar`**

Add:

```tsx
const [preferredMode, setPreferredMode] = useState(readIconSidebarDisplayModePreference)
const viewportForcesCollapsed = useViewportMediaQuery("(max-width: 959px)")
const expanded = preferredMode === "expanded" && !viewportForcesCollapsed

const toggleExpanded = () => {
  const next = preferredMode === "expanded" ? "collapsed" : "expanded"
  try {
    writeIconSidebarDisplayModePreference(next)
    setPreferredMode(next)
  } catch (error) {
    logger.error("ui", "IconSidebar::toggleDisplayMode", "failed to save icon sidebar mode", error)
  }
}
```

Give the root a navigation landmark and deterministic widths:

```tsx
<nav
  aria-label={t("navigation.primary")}
  className={cn(
    "shrink-0 border-r border-border-soft bg-surface-sidebar flex flex-col overflow-hidden",
    "transition-[width] duration-200 ease-out motion-reduce:transition-none",
    expanded ? "w-36" : "w-[76px] items-center",
  )}
>
```

Add the top toggle with `PanelLeftClose` / `PanelLeftOpen`, `IconTip`, and translated `aria-label`. Disable the toggle only while the viewport forces collapse, so clicking cannot misleadingly alter a state that the user cannot see.

- [ ] **Step 5: Convert every existing entry without changing behavior**

Replace repeated `Button` shells with `IconSidebarNavItem`. Pass existing click handlers, context-menu wrappers, badge markup, server status, theme, language, help, settings, and update behaviors through unchanged. Use one neutral active palette for every route; status colors remain semantic. Preserve the three groups from the approved spec and keep the third group bottom-aligned.

The expanded label order must be:

```ts
[
  "会话", "知识空间", "成果", "定时任务", "数据看板",
  "技能广场", "我的技能", "计划", "日志",
  "个人资料", "外观", "语言", "帮助", "设置", "关于",
]
```

- [ ] **Step 6: After user approval, run focused component tests and typecheck**

Run: `pnpm vitest run src/components/common/IconSidebar.test.tsx src/components/common/iconSidebarPreference.test.ts`

Expected: PASS.

Run: `pnpm typecheck`

Expected: exit code 0 with no TypeScript errors.

---

### Task 3: Internationalized labels, purpose tooltips, and visual verification

**Files:**
- Modify: `src/i18n/locales/zh.json`
- Modify: `src/i18n/locales/zh-TW.json`
- Modify: `src/i18n/locales/en.json`
- Modify: `src/components/common/IconSidebar.test.tsx`

**Interfaces:**
- Consumes: `IconSidebarNavItem.tooltip` from Task 2.
- Produces: `navigation.primary`, `navigation.expand`, `navigation.collapse`, and `navigation.purpose.*` translations in all supported languages.

- [ ] **Step 1: Add an i18n completeness test**

Extend `IconSidebar.test.tsx` to render collapsed mode in each supported locale and assert that every navigation button has a non-empty accessible name and every tooltip receives a translated string that contains both a label and purpose separated by `｜`.

```tsx
expect(iconTipLabels).toEqual(
  expect.arrayContaining([
    expect.stringMatching(/^会话｜.+/),
    expect.stringMatching(/^知识空间｜.+/),
    expect.stringMatching(/^设置｜.+/),
  ]),
)
```

- [ ] **Step 2: After user approval, run the test and verify failure**

Run: `pnpm vitest run src/components/common/IconSidebar.test.tsx`

Expected: FAIL because the `navigation.*` translations are absent.

- [ ] **Step 3: Add complete translations**

Add the same key shape to all three locale files. Chinese source copy:

```json
"navigation": {
  "primary": "主要功能",
  "expand": "展开导航",
  "collapse": "收起导航",
  "purpose": {
    "chat": "管理日常对话",
    "knowledge": "整理长期资料",
    "artifacts": "查看生成成果",
    "calendar": "管理定时任务",
    "dashboard": "查看使用数据",
    "skillhub": "发现可用技能",
    "mySkills": "管理已安装技能",
    "plans": "查看计划记录",
    "logs": "查看运行日志",
    "profile": "管理个人资料",
    "theme": "切换界面外观",
    "language": "切换显示语言",
    "help": "打开使用手册",
    "settings": "调整应用设置",
    "about": "查看版本与更新"
  }
}
```

Provide natural Traditional Chinese and English equivalents; do not copy Simplified Chinese into `zh-TW.json`. Build collapsed tooltips as `${label}｜${purpose}` in `IconSidebar.tsx` so punctuation stays consistent.

- [ ] **Step 4: After user approval, run i18n and focused UI checks**

Run: `node scripts/sync-i18n.mjs --check`

Expected: exit code 0 and no missing keys.

Run: `pnpm vitest run src/components/common/IconSidebar.test.tsx`

Expected: PASS.

- [ ] **Step 5: Manually verify the approved desktop states**

Inspect 1024px, 1280px, and 1440px widths plus a 959px-or-narrower window in light and dark themes. Confirm:

- 1024px and wider starts at 144px and has no horizontal overflow.
- 959px and narrower renders 76px without overwriting `hope.iconSidebarDisplayMode`.
- Expanding restores labels in the exact approved order.
- English labels do not overlap icons or badges.
- Hover only changes background/color.
- Unread badges, context menus, language menu, server status, update dot, help, and about remain usable.
- Keyboard Tab order is logical and Enter/Space activates each entry.
- Reduced-motion removes the width transition.

- [ ] **Step 6: Review the final diff without committing**

Run: `git diff --check`

Expected: no whitespace errors.

Run: `git diff -- src/components/common src/components/settings/SettingsResetControl.tsx src/components/settings/SettingsResetControl.test.tsx src/i18n/locales`

Expected: only the approved sidebar feature, preference reset, tests, and translations are present. Stop for user review; do not commit or push.
