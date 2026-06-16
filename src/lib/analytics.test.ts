import { describe, expect, it } from "vitest"

import { track } from "./analytics"

describe("analytics track", () => {
  it("is a no-op that never throws (telemetry removed in the Windows fork)", () => {
    expect(() => {
      track("test_event", { foo: "bar" })
      track("another_event")
    }).not.toThrow()
  })
})
