import { beforeEach, describe, expect, it, vi } from "vitest"
import { makeCtx } from "../test-helpers.js"

const loadPlugin = async () => {
  await import("./plugin.js")
  return globalThis.__openusage_plugin
}

const DAY = 24 * 60 * 60 * 1000
const HOUR = 60 * 60 * 1000

function setupAuth(ctx) {
  ctx.host.fs.writeText(
    "~/.codex/auth.json",
    JSON.stringify({
      tokens: { access_token: "token", account_id: "acct-1" },
      last_refresh: new Date().toISOString(),
    })
  )
}

// A minimal successful usage response; `body` is merged into the JSON payload.
function usageResp(body) {
  return {
    status: 200,
    headers: { "x-codex-primary-used-percent": "12" },
    bodyText: JSON.stringify(body),
  }
}

const isResetUrl = (opts) => opts.url.includes("rate-limit-reset-credits")

describe("codex plugin rate-limit reset credits", () => {
  beforeEach(() => {
    delete globalThis.__openusage_plugin
    vi.resetModules()
  })

  it("lists available reset credits with expiry, soonest first, excluding non-available", async () => {
    const ctx = makeCtx()
    setupAuth(ctx)
    const now = Date.now()
    const soon = new Date(now + 5 * DAY + 2 * HOUR).toISOString()
    const far = new Date(now + 17 * DAY + 2 * HOUR).toISOString()
    const resetsBody = {
      available_count: 2,
      credits: [
        { id: "RateLimitResetCredit_far", status: "available", expires_at: far },
        { id: "RateLimitResetCredit_soon", status: "available", expires_at: soon },
        { id: "RateLimitResetCredit_used", status: "redeemed", expires_at: soon },
      ],
    }
    ctx.host.http.request.mockImplementation((opts) =>
      isResetUrl(opts)
        ? { status: 200, headers: {}, bodyText: JSON.stringify(resetsBody) }
        : usageResp({ rate_limit_reset_credits: { available_count: 2 } })
    )

    const plugin = await loadPlugin()
    const result = plugin.probe(ctx)

    const summary = result.lines.find((l) => l.label === "Rate Limit Resets")
    expect(summary).toMatchObject({ type: "text", value: "2 available", color: "#74AA9C" })

    const resets = result.lines.filter((l) => l.label === "Reset credit")
    expect(resets).toHaveLength(2) // redeemed credit is excluded
    // soonest-expiry credit is listed first
    expect(parseInt(resets[0].value, 10)).toBeLessThan(parseInt(resets[1].value, 10))
    expect(resets[0].value).toMatch(/ left$/)
    expect(resets[0].subtitle).toMatch(/^Expires /)
    expect(resets[1].subtitle).toMatch(/^Expires /)
  })

  it("colors a credit amber when it expires within the soon window", async () => {
    const ctx = makeCtx()
    setupAuth(ctx)
    const now = Date.now()
    const resetsBody = {
      available_count: 1,
      credits: [
        { id: "c1", status: "available", expires_at: new Date(now + 2 * HOUR).toISOString() },
      ],
    }
    ctx.host.http.request.mockImplementation((opts) =>
      isResetUrl(opts)
        ? { status: 200, headers: {}, bodyText: JSON.stringify(resetsBody) }
        : usageResp({ rate_limit_reset_credits: { available_count: 1 } })
    )

    const plugin = await loadPlugin()
    const result = plugin.probe(ctx)
    const reset = result.lines.find((l) => l.label === "Reset credit")
    expect(reset.color).toBe("#f59e0b")
  })

  it("falls back to the usage summary count when the dedicated request fails", async () => {
    const ctx = makeCtx()
    setupAuth(ctx)
    ctx.host.http.request.mockImplementation((opts) =>
      isResetUrl(opts)
        ? { status: 500, headers: {}, bodyText: "oops" }
        : usageResp({ rate_limit_reset_credits: { available_count: 3 } })
    )

    const plugin = await loadPlugin()
    const result = plugin.probe(ctx)
    const summary = result.lines.find((l) => l.label === "Rate Limit Resets")
    expect(summary).toMatchObject({ value: "3 available", color: "#74AA9C" })
    expect(result.lines.filter((l) => l.label === "Reset credit")).toHaveLength(0)
  })

  it("shows 0 available and skips the dedicated request when no credits remain", async () => {
    const ctx = makeCtx()
    setupAuth(ctx)
    ctx.host.http.request.mockImplementation((opts) => {
      if (isResetUrl(opts)) throw new Error("dedicated endpoint should not be called")
      return usageResp({ rate_limit_reset_credits: { available_count: 0 } })
    })

    const plugin = await loadPlugin()
    const result = plugin.probe(ctx)
    const summary = result.lines.find((l) => l.label === "Rate Limit Resets")
    expect(summary).toMatchObject({ value: "0 available" })
    expect(summary.color).toBeUndefined()
    expect(ctx.host.http.request.mock.calls.some(isResetUrlCall)).toBe(false)
  })

  it("omits the resets section entirely when usage has no reset-credit data", async () => {
    const ctx = makeCtx()
    setupAuth(ctx)
    ctx.host.http.request.mockImplementation((opts) => {
      if (isResetUrl(opts)) throw new Error("dedicated endpoint should not be called")
      return usageResp({})
    })

    const plugin = await loadPlugin()
    const result = plugin.probe(ctx)
    expect(result.lines.find((l) => l.label === "Rate Limit Resets")).toBeUndefined()
    expect(ctx.host.http.request.mock.calls.some(isResetUrlCall)).toBe(false)
  })
})

const isResetUrlCall = (call) => isResetUrl(call[0])
