/**
 * Vitest setup file — wires the `axe` accessibility matcher into
 * every test that imports `expect` from vitest.
 *
 * Closes part of EXTERNAL_AUDIT_BACKLOG.md #34 (WCAG 2.2 AA a11y
 * audit).  axe-core covers a meaningful subset of WCAG (~57% of
 * issues per Deque's own measurement) automatically; the remaining
 * issues require manual / AT testing on real hardware (separate
 * pre-release task in `MILESTONES.md`).
 *
 * Usage in any `*.test.tsx`:
 *
 *   import { render } from "@testing-library/react";
 *   import { axe } from "vitest-axe";
 *
 *   it("ErrorBoundary fallback is accessible", async () => {
 *     const { container } = render(<ErrorBoundary><Boom /></ErrorBoundary>);
 *     const result = await axe(container);
 *     expect(result).toHaveNoViolations();
 *   });
 *
 * The `toHaveNoViolations` matcher is registered globally below.
 */

import { expect } from "vitest";
import * as matchers from "vitest-axe/matchers";

expect.extend(matchers);

// Augment vitest's matcher type so TypeScript knows about
// `toHaveNoViolations` without each test having to import the type
// helper.
//
// `Assertion<T = any>` must match @vitest/expect's own declaration
// exactly — TS2428 fires if the default type parameter diverges
// (vitest uses `any`, not `unknown`). See
// @vitest/expect/dist/index.d.ts:`interface Assertion<T = any>`.
declare module "vitest" {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  interface Assertion<T = any> {
    toHaveNoViolations(): T;
  }
  interface AsymmetricMatchersContaining {
    toHaveNoViolations(): unknown;
  }
}
