---
name: vercel-react-best-practices
description: React and Next.js performance optimization guidelines from Vercel Engineering. Contains rules for eliminating waterfalls, bundle size optimization, and more.
---

# React Best Practices

React and Next.js performance optimization guidelines from Vercel Engineering.

## When to Use This Skill

- Writing new React components or Next.js pages
- Implementing data fetching (client or server-side)
- Reviewing code for performance issues
- Optimizing bundle size or load times

## Rules & Guidelines

### 1. Eliminating Waterfalls (Critical)

- **Problem:** Sequential async operations slow down page loads.
- **Rule:** `rules/async-parallel.md`
- **Solution:** Use `Promise.all()` for independent operations.

### 2. Bundle Size Optimization (Critical)

- **Problem:** Large JS bundles delay interactivity.
- **Rule:** `rules/bundle-barrel-imports.md`
- **Solution:** Avoid barrel imports; import directly from source or use `optimizePackageImports`.

## Usage

When reviewing or writing code, check against these rules:

1.  **Async Logic:** Are multiple `await` calls sequential when they could be parallel?
2.  **Imports:** Are we importing entire libraries (e.g. `lucide-react`) instead of specific modules?

## References

- [Vercel Agent Skills Repo](https://github.com/vercel-labs/agent-skills)
