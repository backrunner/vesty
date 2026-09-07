# Vesty documentation site

The English and Simplified Chinese documentation uses **Svedocs 0.2.1** and **svedocs-cli 0.2.1**, installed from npm. A sibling Svedocs checkout is not required.

## Develop and verify

Use Node.js 24+ and the package manager pinned in `package.json` (`pnpm@12.3.4`).

```bash
cd docs
pnpm install --frozen-lockfile
pnpm dev
```

Before submitting a change:

```bash
pnpm check
pnpm check:ui
pnpm build:static
```

`pnpm check` validates content, internal links, assets, and translation coverage. The production build also compiles the Svelte site and prerenders its routes. Check the English and Chinese homepages, a guide, search, the mobile menu, and both color modes in a browser.

## Build and deploy

| Command | Output / use |
| --- | --- |
| `pnpm build:static` | Prerendered site in `build/`; upload to a static host |
| `pnpm build:edge` | Cloudflare output in `.svelte-kit/cloudflare/` |
| `pnpm build:spa` | Static SPA with a `200.html` fallback |
| `pnpm preview` | Preview the last production build locally |

The default `pnpm build` uses the edge adapter. These commands build artifacts; they do not publish the site or configure DNS.

Set `VESTY_DOCS_URL` to the site's public origin when checking or building for production. For example, `VESTY_DOCS_URL=https://your-docs-domain.example pnpm build:static`. Use the actual domain, without a route suffix. Without it, local development still works, but Svedocs warns that absolute sitemap URLs are unavailable.

Build scripts pass `--no-og` because the SvelteKit OG endpoint already renders the configured images. This avoids a duplicate generated `static/og` tree colliding with prerender entries. Search is local and AI is disabled, so a static deployment does not need an AI service or server-side search credentials.

## Content and localization

- English: `content/docs/` and `content/pages/`.
- Simplified Chinese: matching paths under `content/docs/zh/` and `content/pages/zh/`.
- Keep page slugs and guide ordering aligned between languages.
- Use the existing frontmatter fields: `title`, `description`, and `order`.
- Site-relative links use `/docs/...` in English and `/docs/zh/...` in Chinese.
- Shared UI and landing translations live in `svedocs.config.ts`.
- Landing links use Svedocs' `resolveLocalizedHref` so navigation preserves the active locale.

## Theme and identity

Vesty's site uses a modern terminal style: a near-black blue background, green command accents, cyan signal traces, monospace controls, and fine grid details. The logo is deliberately monochrome: two solid strokes form an asymmetric V, separated by a narrow diagonal gap. The lowercase wordmark uses custom vector letterforms.

| Asset / source | Purpose |
| --- | --- |
| `static/brand/vesty-mark.svg` | Canonical monochrome V symbol on a transparent background |
| `static/brand/vesty-wordmark.svg` | Lowercase wordmark drawn as paths; no font dependency |
| `static/brand/vesty-identity.svg` | Black/white and small-size identity reference sheet |
| `static/favicon.svg` | Same geometry with automatic light/dark color for browser tabs |
| `static/brand/vesty-banner.svg` | Standalone README banner with accessible SVG title and description |
| `src/lib/Landing.svelte` | Bilingual homepage, template selector, command copy, architecture, and guide entry points |
| `src/lib/Brand.svelte` | Localized navigation lockup using the symbol and wordmark |
| `src/lib/styles/vesty.css` | Site tokens, reading styles, responsive landing layout, and focus treatment |
| `svedocs.config.ts` | Branding, font stacks, translations, navigation, search, and SEO |
| `src/routes/+layout.svelte` | Imports Svedocs base theme followed by Vesty overrides |

Use the symbol and wordmark in one color: dark on a light background, white on a dark background. Preserve the diagonal gap and the supplied proportions; do not add a container, cursor, gradient, or colored second stroke. Keep clear space around the mark. Logo assets are pure SVG paths; the banner's supporting text uses system fonts. No external font download or raster generation service is required.

The homepage terminal previews real scaffold commands and selected project files. Switching the template updates the preview; the copy button copies the displayed command. It does not run shell commands. The signal trace is a static illustration: it runs no animation loop, starts no audio context, and makes no audio requests. The reading theme follows system preference and offers a persistent manual switch. Avoid adding continuous animation or inaccessible hover-only controls.

## Upgrade Svedocs

Upgrade the framework and CLI together, keeping exact versions in `package.json`, then commit the updated `pnpm-lock.yaml`. Review any version-specific entries in `pnpm-workspace.yaml` too. Run content checks, static and edge builds, then verify navigation, search, localization, and theme switching in the browser.
