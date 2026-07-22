// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react"
import { afterEach, expect, test, vi } from "vitest"

import MySkillList from "./MySkillList"
import type { MySkillCloudEntry } from "@/types/skillhub"

afterEach(() => {
  cleanup()
})

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}))

const row: MySkillCloudEntry = {
  local: {
    name: "Code Review",
    description: "Review code changes",
    source: "user",
    base_dir: "C:/skills/code-review",
    enabled: true,
    requires_env: [],
    display: {
      tags: ["code", "review", "quality", "extra"],
      emoji: "🔍",
    },
  },
  cloud: {
    registrySlug: "alice/code-review",
    publishState: "pending-review",
  },
}

const disabledRow: MySkillCloudEntry = {
  local: {
    name: "Offline Helper",
    description: "Helps offline",
    source: "user",
    base_dir: "C:/skills/offline-helper",
    enabled: false,
    requires_env: [],
  },
  cloud: {
    registrySlug: "alice/offline-helper",
    publishState: "not-published",
  },
}

const publishedRow: MySkillCloudEntry = {
  local: {
    name: "Published Helper",
    description: "Already published",
    source: "user",
    base_dir: "C:/skills/published-helper",
    enabled: true,
    requires_env: [],
  },
  cloud: {
    registrySlug: "alice/published-helper",
    publishState: "published",
  },
}

test("MySkillList renders card grid without registry slug and hides pending submit", () => {
  render(
    <MySkillList
      rows={[row]}
      loading={false}
      canUseCloudActions
      onRefresh={vi.fn()}
      onSubmitReview={vi.fn()}
      onOpenSkill={vi.fn()}
      onOpenDir={vi.fn()}
    />,
  )

  expect(screen.getByText("Code Review")).toBeTruthy()
  expect(screen.queryByText("alice/code-review")).toBeNull()
  expect(screen.getByText("mySkills.state.pendingReview")).toBeTruthy()
  // At most 3 tags on the card surface.
  expect(screen.getByText("code")).toBeTruthy()
  expect(screen.getByText("review")).toBeTruthy()
  expect(screen.getByText("quality")).toBeTruthy()
  expect(screen.queryByText("extra")).toBeNull()
  // Pending review hides the primary submit button.
  expect(screen.queryByLabelText("mySkills.submitReview")).toBeNull()
})

test("MySkillList uses publish filters when logged in", () => {
  const onRefresh = vi.fn(async () => undefined)
  const { container } = render(
    <MySkillList
      rows={[row, disabledRow, publishedRow]}
      loading={false}
      canUseCloudActions
      onRefresh={onRefresh}
      onSubmitReview={vi.fn()}
      onOpenSkill={vi.fn()}
      onOpenDir={vi.fn()}
    />,
  )
  const view = within(container)

  expect(view.getByRole("button", { name: /mySkills\.filter\.local/ })).toBeTruthy()
  expect(view.getByRole("button", { name: /mySkills\.filter\.pendingReview/ })).toBeTruthy()
  expect(view.getByRole("button", { name: /mySkills\.filter\.published/ })).toBeTruthy()

  fireEvent.click(view.getByRole("button", { name: /mySkills\.filter\.published/ }))
  expect(view.getByText("Published Helper")).toBeTruthy()
  expect(view.queryByText("Code Review")).toBeNull()
  expect(view.queryByText("Offline Helper")).toBeNull()

  fireEvent.click(view.getByRole("button", { name: /mySkills\.refreshStatus/ }))
  expect(onRefresh).toHaveBeenCalled()
})

test("MySkillList filters by local disabled state when logged out", () => {
  const onRefresh = vi.fn(async () => undefined)
  const { container } = render(
    <MySkillList
      rows={[row, disabledRow]}
      loading={false}
      canUseCloudActions={false}
      onRefresh={onRefresh}
      onSubmitReview={vi.fn()}
      onOpenSkill={vi.fn()}
      onOpenDir={vi.fn()}
    />,
  )
  const view = within(container)

  fireEvent.click(view.getByRole("button", { name: /mySkills\.filter\.disabled/ }))
  expect(view.getByText("Offline Helper")).toBeTruthy()
  expect(view.queryByText("Code Review")).toBeNull()

  fireEvent.click(view.getByRole("button", { name: /mySkills\.refreshLocal/ }))
  expect(onRefresh).toHaveBeenCalled()
})

test("MySkillList search matches description but not registry slug", () => {
  const { container } = render(
    <MySkillList
      rows={[row, disabledRow]}
      loading={false}
      canUseCloudActions
      onRefresh={vi.fn()}
      onSubmitReview={vi.fn()}
      onOpenSkill={vi.fn()}
      onOpenDir={vi.fn()}
    />,
  )
  const view = within(container)

  const input = view.getByLabelText("mySkills.searchPlaceholder")
  fireEvent.change(input, { target: { value: "offline" } })
  expect(view.getByText("Offline Helper")).toBeTruthy()
  expect(view.queryByText("Code Review")).toBeNull()

  fireEvent.change(input, { target: { value: "alice/code-review" } })
  expect(view.getByText("mySkills.noMatches")).toBeTruthy()
})
