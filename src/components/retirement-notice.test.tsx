import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi, beforeEach } from "vitest"

import { RetirementNotice } from "@/components/retirement-notice"

const openerState = vi.hoisted(() => ({
  openUrlMock: vi.fn(() => Promise.resolve()),
}))

const settingsState = vi.hoisted(() => ({
  loadMock: vi.fn<[], Promise<number | null>>(() => Promise.resolve(null)),
  saveMock: vi.fn(() => Promise.resolve()),
}))

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: openerState.openUrlMock,
}))

vi.mock("@/lib/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/settings")>()
  return {
    ...actual,
    loadRetirementNoticeDismissedAt: settingsState.loadMock,
    saveRetirementNoticeDismissedAt: settingsState.saveMock,
  }
})

describe("RetirementNotice", () => {
  beforeEach(() => {
    openerState.openUrlMock.mockClear()
    settingsState.loadMock.mockReset()
    settingsState.loadMock.mockResolvedValue(null)
    settingsState.saveMock.mockClear()
  })

  it("shows the notice when never dismissed", async () => {
    render(<RetirementNotice />)
    expect(await screen.findByText("OpenUsage Has Moved")).toBeInTheDocument()
  })

  it("stays hidden when dismissed within the interval", async () => {
    settingsState.loadMock.mockResolvedValue(Date.now())
    render(<RetirementNotice />)
    await waitFor(() => expect(settingsState.loadMock).toHaveBeenCalled())
    expect(screen.queryByText("OpenUsage Has Moved")).not.toBeInTheDocument()
  })

  it("opens the new app link", async () => {
    render(<RetirementNotice />)
    await screen.findByText("OpenUsage Has Moved")
    await userEvent.click(screen.getByRole("button", { name: "Get the New App" }))
    expect(openerState.openUrlMock).toHaveBeenCalledWith("https://www.openusage.ai")
  })

  it("persists a timestamp and hides on dismiss", async () => {
    render(<RetirementNotice />)
    await screen.findByText("OpenUsage Has Moved")
    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }))
    expect(screen.queryByText("OpenUsage Has Moved")).not.toBeInTheDocument()
    await waitFor(() => expect(settingsState.saveMock).toHaveBeenCalledTimes(1))
    expect(typeof settingsState.saveMock.mock.calls[0][0]).toBe("number")
  })

  it("shows the notice when load fails (fail open)", async () => {
    settingsState.loadMock.mockRejectedValue(new Error("boom"))
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {})
    render(<RetirementNotice />)
    expect(await screen.findByText("OpenUsage Has Moved")).toBeInTheDocument()
    errorSpy.mockRestore()
  })
})
