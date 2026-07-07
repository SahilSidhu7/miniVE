# Manual release checklist

Run with Docker Desktop up, `npm run tauri dev`:

- [ ] Docker stopped → gate screen appears; start Docker → Re-check enters app
- [ ] Create Python 3.12 env with port 8000→8000
- [ ] Create Node 20 env, no ports
- [ ] Clone a public repo into the Node env; folder appears in file tree
- [ ] Upload a file into the Python env; visible in tree and `ls /workspace`
- [ ] Terminal: `pip install requests` persists after stop/start of env
- [ ] `python -m http.server 8000` → preview pane renders listing
- [ ] Stop env from Docker Desktop → card flips to stopped (event stream works)
- [ ] Delete both envs → `docker ps -a` and `docker volume ls` show no minive-* leftovers

## Tracked hardening

- [ ] Replace "csp": null with a scoped policy ("default-src 'self'; frame-src http://localhost:*") and verify the preview iframe + app styles still work in a dev run
