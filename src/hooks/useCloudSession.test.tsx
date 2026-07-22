// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react"
import { beforeEach, expect, test, vi } from "vitest"

import { useCloudSession } from "./useCloudSession"
import type { CloudSession } from "@/types/skillhub"

const callMock = vi.fn()

vi.mock("@/lib/transport-provider", () => ({
  getTransport: () => ({
    call: callMock,
  }),
}))

const aliceSession: CloudSession = {
  serverUrl: "http://127.0.0.1:3000",
  user: { id: "u1", username: "alice" },
  authenticated: true,
}

beforeEach(() => {
  callMock.mockReset()
})

test("updates other hook instances after cloud login", async () => {
  callMock.mockImplementation(async (command: string) => {
    if (command === "cloud_get_session") return null
    if (command === "cloud_login") return aliceSession
    throw new Error(`unexpected command ${command}`)
  })

  const first = renderHook(() => useCloudSession())
  const second = renderHook(() => useCloudSession())

  await waitFor(() => {
    expect(first.result.current.loading).toBe(false)
    expect(second.result.current.loading).toBe(false)
  })

  await act(async () => {
    await first.result.current.login({
      serverUrl: "http://127.0.0.1:3000",
      username: "alice",
      password: "secret",
    })
  })

  await waitFor(() => {
    expect(second.result.current.session?.user.username).toBe("alice")
  })
})
