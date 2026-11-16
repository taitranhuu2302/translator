# Tailwind and shadcn/ui are the UI's foundation

`DESIGN.md` describes the app in shadcn/ui's own vocabulary — `--background`, `--card`, `--primary`, a `0.75rem` radius — and the code had none of it. The UI was one hand-authored stylesheet with six ad-hoc variables and seven different corner radii, following the system appearance rather than the ocean palette.

The first planning pass deliberately chose *not* to adopt the framework: hand-written CSS with shadcn-shaped token names, with the markup left intact. At seven components, the reasoning went, a framework buys little and the rewrite risks behaviour that is already tested.

That was reversed on the user's direction. The cost of a home-made approximation is paid later, by every component added after this one: it will be hand-built, and it will differ from its neighbours in the small ways this redesign exists to remove. A real component system means a new control is a CLI command and an inherited set of states, not a new decision.

**Decision.** Tailwind CSS v4 (CSS-first, `@theme`, no config file) plus shadcn/ui, initialised inside the `ui` workspace. Tokens carry `DESIGN.md`'s HSL values verbatim rather than shadcn's `oklch` defaults. Dark-only. The Quick Translate popup is composed from native elements and excluded from Radix, because a portal dropdown clips in a 420px window.

**Consequences:** the UI markup is rewritten in utility classes and `styles.css` is deleted rather than wrapped — a surviving second stylesheet would recreate the drift being removed. Dependencies and bundle size grow by Tailwind and Radix. Every CLI invocation must name the workspace (`npx shadcn@latest … -c ui`), because the repository root is an npm workspace and the CLI refuses to guess. The shadcn MCP server already configured in `.mcp.json` becomes usable, since it needs `components.json` to exist. Design review moves to a dev-only preview harness, because the agent builds and signs the bundle but never launches it. ADR-0005 is unaffected: transparency and content-driven height are window and content properties, and the panel keeps its non-activating style mask and its first-responder hand-off.
