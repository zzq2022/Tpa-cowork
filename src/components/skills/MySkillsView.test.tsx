// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { expect, test, vi } from "vitest"

import MySkillsView from "./MySkillsView"
import type { MySkillCloudEntry } from "@/types/skillhub"

const transportCallMock = vi.fn()

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock("@/components/cloud/CloudLoginDialog", () => ({
  default: () => <div data-testid="cloud-login-dialog" />,
}))

vi.mock("@/hooks/useCloudSession", () => ({
  useCloudSession: () => ({
    loading: false,
    session: { authenticated: true },
    reload: vi.fn(),
  }),
}))

const rows: MySkillCloudEntry[] = [
  {
    local: {
      name: "autoplan",
      description: "Run planning reviews",
      source: "managed",
      base_dir: "C:/skills/autoplan",
      enabled: true,
      requires_env: [],
    },
    cloud: {
      registrySlug: "alice/autoplan",
      publishState: "not-published",
    },
  },
]

vi.mock("@/hooks/useMySkills", () => ({
  useMySkills: () => ({
    skills: rows,
    skillStatuses: [],
    loading: false,
    error: null,
    reload: vi.fn(),
    refreshLocal: vi.fn(),
    refreshCloud: vi.fn(),
    submitReview: vi.fn(),
  }),
}))

vi.mock("@/lib/transport-provider", () => ({
  getTransport: () => ({
    call: transportCallMock,
  }),
}))

test("MySkillsView opens the shared skill detail view when a skill row is clicked", async () => {
  transportCallMock.mockImplementation((command: string) => {
    if (command === "get_skill_detail") {
      return Promise.resolve({
        name: "autoplan",
        description: "Run planning reviews",
        source: "managed",
        file_path: "C:/skills/autoplan/SKILL.md",
        base_dir: "C:/skills/autoplan",
        content: "# Autoplan\n\nPlan with reviews.",
        enabled: true,
        files: [],
        requires: { bins: [], env: [], os: [] },
      })
    }
    if (command === "get_skill_env") return Promise.resolve({})
    if (command === "get_skills_env_status") return Promise.resolve({})
    if (command === "get_skills_status") return Promise.resolve([])
    return Promise.resolve(undefined)
  })

  render(<MySkillsView onBack={vi.fn()} />)

  const skillButton = screen.getByText("autoplan").closest("button")
  expect(skillButton).toBeTruthy()
  // List cards open the shared detail view on double-click.
  fireEvent.doubleClick(skillButton!)

  await waitFor(() => {
    expect(transportCallMock).toHaveBeenCalledWith("get_skill_detail", { name: "autoplan" })
  })
  expect(await screen.findByRole("heading", { name: "autoplan" })).toBeTruthy()
  expect(screen.getByRole("heading", { name: "Autoplan" })).toBeTruthy()
  expect(screen.getByText("Plan with reviews.")).toBeTruthy()
})
