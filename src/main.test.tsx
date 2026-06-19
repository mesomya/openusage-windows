import { describe, expect, it, vi } from "vitest"

const renderMock = vi.fn()
const createRootMock = vi.fn(() => ({ render: renderMock }))

vi.mock("react-dom/client", () => ({
  default: {
    createRoot: createRootMock,
  },
}))

describe("main", () => {
  // Importing main pulls in the whole app graph; under full-suite CPU load this
  // can exceed the 5s default, so give it headroom to avoid a spurious timeout.
  it("mounts app", async () => {
    document.body.innerHTML = '<div id="root"></div>'
    await import("@/main")
    expect(createRootMock).toHaveBeenCalled()
    expect(renderMock).toHaveBeenCalled()
  }, 30000)
})
