# Previewing the demo site

Run the static site locally from the repository root:

```bash
cd site
python3 -m http.server 8000 --bind 127.0.0.1
```

Then open `http://127.0.0.1:8000/`. The OpenAI Sites URL is a private,
owner-only preview for commercial-team review. GitHub Pages uploads `site/`
directly, and the private preview is packaged from that same committed
directory.

`.openai/hosting.json` declares `"directory": "dist"`. `dist/` is the hosting
archive's build-output path, not a second authoring directory. The repository
currently contains no build command or configuration that records how
`site/` is copied into `dist/`, and the available deployment metadata does not
record that copy step. The current preview was packaged from the committed
`site/` content, but future publishers must verify that staging relationship
rather than infer it from the hosting manifest.
