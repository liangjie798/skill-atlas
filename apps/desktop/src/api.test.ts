import { beforeAll, describe, expect, it } from "vitest";
import { api } from "./api";

beforeAll(() => {
  Object.defineProperty(globalThis, "window", { value: {}, configurable: true });
});

describe("browser demo inventory", () => {
  it("filters skills by text", async () => {
    const result = await api.listSkills({ search: "PDF" });
    expect(result.total).toBe(1);
    expect(result.items[0].name).toBe("pdf");
  });

  it("exposes logical assets separately from installations", async () => {
    const summary = await api.dashboardSummary();
    expect(summary.assets).toBe(3);
    expect(summary.installations).toBe(5);
  });
});
