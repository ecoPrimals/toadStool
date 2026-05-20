# sporeprint/

sporePrint content directory for ToadStool.

Files in this directory are automatically published to
[primals.eco](https://primals.eco) by the sporePrint pipeline when
`notify-sporeprint.yml` fires on push to `main`.

## How it works

1. Push to `main` triggers `.github/workflows/notify-sporeprint.yml`
2. sporePrint clones this repo and copies `sporeprint/*.md`
3. Content is validated (front matter, taxonomies, links)
4. Validated content is published to the primals.eco lab surface

## Files

| File | Purpose |
|------|---------|
| `validation-summary.md` | Primal status, test counts, capabilities, hardware substrates |

## Adding content

Add `.md` files with Zola-compatible `+++` front matter. Include
`[taxonomies]` with `primals` and `springs` arrays for cross-referencing.
