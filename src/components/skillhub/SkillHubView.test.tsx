// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, expect, test, vi } from "vitest"

import SkillHubView from "./SkillHubView"
import type { MySkillCloudEntry } from "@/types/skillhub"

const callMock = vi.fn()

afterEach(() => {
  cleanup()
  callMock.mockReset()
})

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock("@/lib/transport-provider", () => ({
  getTransport: () => ({
    call: callMock,
  }),
}))

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

function page(items: Array<Record<string, unknown>>) {
  return {
    items,
    total: items.length,
    pageSize: 12,
  }
}

const publicSkill = {
  id: "skill-2",
  name: "autoplan",
  description: "Plan with reviews",
  registrySlug: "alice/autoplan",
  authorUsername: "alice",
  downloads: 1,
  stars: 0,
  views: 2,
  updatedAt: "2026-07-01T00:00:00Z",
}

test("SkillHubView loads public skills on mount", async () => {
  callMock.mockResolvedValueOnce(
    page([
      {
        id: "skill-1",
        name: "SQL Helper",
        description: "Write SQL faster",
        registrySlug: "bob/sql-helper",
        authorUsername: "bob",
        downloads: 12,
        stars: 3,
        views: 40,
      },
    ]),
  )

  render(<SkillHubView onBack={vi.fn()} />)

  await waitFor(() => {
    expect(callMock).toHaveBeenCalledWith(
      "skillhub_search_public",
      expect.objectContaining({
        request: expect.objectContaining({ page: 1 }),
      }),
    )
  })
  expect(await screen.findByText("SQL Helper")).toBeTruthy()
})

test("SkillHubView reports the installed skill after a successful download", async () => {
  const installed: MySkillCloudEntry = {
    local: {
      name: "autoplan",
      description: "Plan with reviews",
      source: "managed",
      base_dir: "C:/skills/autoplan",
      enabled: true,
      requires_env: [],
    },
    cloud: {
      registrySlug: "alice/autoplan",
      publishState: "not-published",
    },
  }
  const onDownloaded = vi.fn()
  callMock
    .mockResolvedValueOnce(page([publicSkill])) // mount search
    .mockResolvedValueOnce([]) // local list precheck
    .mockResolvedValueOnce(installed) // download

  render(<SkillHubView onBack={vi.fn()} onDownloaded={onDownloaded} />)
  await screen.findByText("autoplan")

  fireEvent.click(screen.getByRole("button", { name: "skillhub.download" }))

  await waitFor(() => {
    expect(callMock).toHaveBeenCalledWith("skillhub_download_skill", {
      skillId: "skill-2",
      overwrite: false,
    })
  })
  expect(onDownloaded).toHaveBeenCalledWith(installed)
})

test("SkillHubView cancels download when an existing local skill overwrite is declined", async () => {
  callMock
    .mockResolvedValueOnce(page([publicSkill])) // mount search
    .mockResolvedValueOnce([
      {
        local: {
          name: "autoplan",
          description: "Local copy",
          source: "managed",
          base_dir: "C:/skills/autoplan",
          enabled: true,
          requires_env: [],
          display: { version: "1.1.0" },
        },
        cloud: {
          registrySlug: "alice/autoplan",
          publishState: "not-published",
        },
      },
    ])

  render(<SkillHubView onBack={vi.fn()} />)
  await screen.findByText("autoplan")

  fireEvent.click(screen.getByRole("button", { name: "skillhub.download" }))
  await screen.findByText("skillhub.overwriteTitle")
  expect(screen.getByText("skillhub.freshness.unknown")).toBeTruthy()

  fireEvent.click(screen.getByRole("button", { name: "common.cancel" }))

  await waitFor(() => {
    expect(
      callMock.mock.calls.some((call) => call[0] === "skillhub_download_skill"),
    ).toBe(false)
  })
})

test("SkillHubView confirms overwrite for an existing local skill", async () => {
  const installed: MySkillCloudEntry = {
    local: {
      name: "autoplan",
      description: "Plan with reviews",
      source: "managed",
      base_dir: "C:/skills/autoplan",
      enabled: true,
      requires_env: [],
    },
    cloud: {
      registrySlug: "alice/autoplan",
      publishState: "not-published",
    },
  }
  const onDownloaded = vi.fn()
  callMock
    .mockResolvedValueOnce(page([publicSkill])) // mount search
    .mockResolvedValueOnce([
      {
        local: {
          name: "autoplan",
          description: "Local copy",
          source: "managed",
          base_dir: "C:/skills/autoplan",
          enabled: true,
          requires_env: [],
          display: { version: "1.0.0" },
        },
        cloud: {
          registrySlug: "alice/autoplan",
          publishState: "not-published",
        },
      },
    ])
    .mockResolvedValueOnce(installed)

  render(<SkillHubView onBack={vi.fn()} onDownloaded={onDownloaded} />)
  await screen.findByText("autoplan")

  fireEvent.click(screen.getByRole("button", { name: "skillhub.download" }))
  await screen.findByText("skillhub.overwriteTitle")
  fireEvent.click(screen.getByRole("button", { name: "skillhub.overwriteConfirm" }))

  await waitFor(() => {
    expect(callMock).toHaveBeenCalledWith("skillhub_download_skill", {
      skillId: "skill-2",
      overwrite: true,
    })
  })
  expect(onDownloaded).toHaveBeenCalledWith(installed)
})
