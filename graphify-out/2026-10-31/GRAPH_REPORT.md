# Graph Report - clawshell  (2026-10-30)

## Corpus Check
- Corpus is ~699 words - fits in a single context window. You may not need a graph.

## Summary
- 85 nodes · 63 edges · 29 communities (5 shown, 24 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Tauri App Config
- SvelteKit Layout & Modules
- Server Hooks & i18n
- Vitest Example Specs
- Greet Frontend Page
- UI Component Utilities
- Tauri Capabilities
- Build Config
- Bundle Config
- SSR Layout
- Project Root
- App HTML Shell
- Icon 128x128
- Icon 128x128@2x
- Icon 32x32
- Icon Base
- Icon Square107
- Icon Square142
- Icon Square150
- Icon Square284
- Icon Square30
- Icon Square310
- Icon Square44
- Icon Square71
- Icon Square89
- Icon Store Logo

## God Nodes (most connected - your core abstractions)
1. `build` - 5 edges
2. `bundle` - 4 edges
3. `greet()` - 3 edges
4. `app` - 3 edges
5. `security` - 2 edges
6. `handle` - 1 edges
7. `WithoutChild` - 1 edges
8. `WithoutChildren` - 1 edges
9. `WithoutChildrenOrChild` - 1 edges
10. `WithElementRef` - 1 edges

## Surprising Connections (you probably didn't know these)
- None detected - all connections are within the same source files.

## Import Cycles
- None detected.

## Communities (29 total, 24 thin omitted)

### Community 0 - "Tauri App Config"
Cohesion: 0.22
Nodes (8): app, security, windows, identifier, productName, $schema, csp, version

### Community 5 - "UI Component Utilities"
Cohesion: 0.33
Nodes (4): WithElementRef, WithoutChild, WithoutChildren, WithoutChildrenOrChild

### Community 6 - "Tauri Capabilities"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 7 - "Build Config"
Cohesion: 0.40
Nodes (5): build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist

### Community 8 - "Bundle Config"
Cohesion: 0.50
Nodes (4): bundle, active, icon, targets

## Knowledge Gaps
- **18 isolated node(s):** `$app/types`, `$lib/paraglide/runtime.js`, `clawshell`, `src/app.html (SvelteKit app shell)`, `128x128 app icon` (+13 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 65 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **24 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `build` connect `Build Config` to `Tauri App Config`?**
  _High betweenness centrality (0.017) - this node is a cross-community bridge._
- **What connects `$app/types`, `$lib/paraglide/runtime.js`, `clawshell` to the rest of the system?**
  _18 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Why does `bundle` connect `Bundle Config` to `Tauri App Config`?**
  _High betweenness centrality (0.013) - this node is a cross-community bridge._