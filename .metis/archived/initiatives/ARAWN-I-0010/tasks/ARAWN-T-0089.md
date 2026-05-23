---
id: marketplace-system-marketplace
level: task
title: "Marketplace system — marketplace sources, known_marketplaces.json, fetch and parse marketplace manifests"
short_code: "ARAWN-T-0089"
created_at: 2026-04-04T13:05:28.375646+00:00
updated_at: 2026-04-04T13:35:48.993707+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Marketplace system — marketplace sources, known_marketplaces.json, fetch and parse marketplace manifests

## Objective

Build the marketplace system: source types, `known_marketplaces.json` registry, fetching marketplace manifests from GitHub/git repos, and parsing the `marketplace.json` format to discover available plugins.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MarketplaceSource` enum: `GitHub { repo }`, `Git { url }`, `Directory { path }` — each with optional `ref` (branch/tag)
- [ ] `MarketplaceManifest` struct: name, plugins list (name, version, description, source)
- [ ] `MarketplaceEntry` struct: source, install_location, last_updated — stored in known_marketplaces.json
- [ ] `known_marketplaces.json` read/write: `HashMap<String, MarketplaceEntry>`
- [ ] `fetch_marketplace(source) -> MarketplaceManifest`: clone GitHub repo to `~/.arawn/plugins/marketplaces/{name}/`, read `.claude-plugin/marketplace.json`
- [ ] GitHub source: `git clone --depth 1` the repo (or `git pull` if already cloned)
- [ ] Git source: same as GitHub but with arbitrary URL
- [ ] Directory source: read marketplace.json directly from local path
- [ ] `add_marketplace(name, source)`: fetch + register in known_marketplaces.json
- [ ] `list_marketplaces()`: return all registered marketplaces with their plugin catalogs
- [ ] `resolve_plugin(name, marketplace)`: find a plugin entry in a marketplace manifest
- [ ] Unit tests: parse marketplace manifest, add/list marketplaces, resolve plugin by name
- [ ] Integration test: clone a local git repo as marketplace, verify manifest loads

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/marketplace.rs`
- Use `std::process::Command` for git operations (git clone, git pull)
- Marketplace manifest lives at `.claude-plugin/marketplace.json` inside the cloned repo
- Depends on T-0088 for PluginIdentifier and cache structure

## Status Updates

*To be added during implementation*