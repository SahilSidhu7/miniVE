# SEO / content plan

Goal: own the "disposable dev environment" niche in search. Small niche, weak
competition — winnable by an indie project. Everything publishes to dev.to or a
`/blog` section of the landing page (dev.to is fine to start: zero setup,
built-in audience, canonical URL support).

## Keyword targets

**Primary (page titles, H1s):**
- disposable dev environment
- isolated development environment
- docker dev environment gui
- try code without installing / run untrusted code safely

**Secondary (in-body, FAQ):**
- docker desktop dev environment
- local alternative to codespaces / gitpod alternative local
- python sandbox environment desktop
- test npm package safely
- clean up docker containers and volumes

**Long-tail (one post each — see calendar):**
- how to run someone else's github repo safely
- multiple python versions without pyenv
- multiple node versions without nvm

## On-page basics (landing repo)

- [x] descriptive `<title>` + meta description on index and docs pages
- [ ] add JSON-LD `SoftwareApplication` schema to index.html once download counts exist
- [ ] add og:image (use gallery still S3) for link previews on social — biggest CTR lever
- [ ] GitHub repo: set Description + Website + Topics (docker, devtools, tauri,
      sandbox, development-environment) — repo pages rank surprisingly well

## Content calendar (1 post ≈ every 2 weeks; each ends with a soft miniVE CTA)

1. **"How to run someone else's GitHub repo without wrecking your machine"**
   — the flagship how-to; covers manual docker way first, then miniVE. Targets the
   long-tail query with real volume.
2. **"Stop stacking Python versions: pyenv vs docker vs miniVE"**
   — comparison post; targets "multiple python versions" queries.
3. **"nvm fatigue: a saner way to juggle Node versions"** — same play for Node.
4. **"Cloud IDEs vs local isolation: what Codespaces costs you"**
   — targets "codespaces alternative"; honest comparison (cloud is right for teams;
   local is right for experiments).
5. **"What `docker run -it` doesn't clean up (and how to check)"**
   — the volume-leak education post; establishes the cleanup credibility that is
   miniVE's core promise.
6. **"Anatomy of a Tauri app that drives Docker"** — engineering deep-dive
   (bollard, terminal streams, event reconnects). This one is for HN/lobsters, not
   search — engineering posts earn the backlinks that make posts 1–5 rank.

## Measurement

- GitHub stars + release download counts (free, in repo Insights/API)
- Landing page: add a privacy-friendly counter only if curiosity demands
  (GoatCounter, free tier, no cookies) — skip until it matters.
- Search Console on the landing domain once Pages is live — free, shows which
  queries actually land.
