# Clifacet Documentation

Fumadocs documentation site for the Clifacet CLI framework.

## Local Development

Install dependencies and start the development server:

```sh
bun install
bun run dev
```

Or through Pixi:

```sh
pixi run docs-dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser.

## Static Export Build

Build the static HTML export for GitHub Pages deployment:

```sh
bun run build
```

Or through Pixi:

```sh
pixi run docs-build
```

The exported site is placed in `out/`.
