# Vesty documentation site

This SvelteKit site is built with the current local svedocs development version:

```text
svedocs 0.1.0-beta.7
source: ../svedocs @ 29cb560
```

The package links in `package.json` intentionally point at the sibling `../svedocs` checkout until beta.7 is published. Build the sibling packages first when their `dist/` output is missing.

```bash
pnpm --dir ../svedocs --filter svedocs build
pnpm --dir ../svedocs --filter svedocs-cli build

cd docs
pnpm install
pnpm dev
```

Useful checks:

```bash
pnpm check
pnpm build:static
```

The build scripts pass `--no-og` because the SvelteKit OG endpoint already renders the configured images. This avoids writing a duplicate `static/og` tree that would collide with dynamic prerender entries on the next build.

English content lives directly under `content/docs` and `content/pages`. Simplified Chinese mirrors it under the respective `zh/` directories.
