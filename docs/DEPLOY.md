# Ghost Production Deployment

Deploy scribe-api (Docker) on your server. Users download the Scribe desktop app from ghost.exora.solutions and connect to your hosted API.

## Architecture

- **ghost.exora.solutions**: Web UI + admin-api (landing, registration, subscriptions, download page)
- **api.ghost.exora.solutions** (or your subdomain): scribe-api in Docker
- **Users**: Download Scribe from the website, install, connect to your scribe-api

---

## 1. Deploy scribe-api (Docker) on Your Server

### Prerequisites

- Docker and Docker Compose
- DNS: A record for `api.ghost.exora.solutions` pointing to your server IP

### Steps

```bash
# 1. Clone or copy the project to your server
cd /var/www/ghost/ghost_main  # or your path

# 2. Create .env
cp docker/.env.example .env
# Edit .env:
#   API_ACCESS_KEY=<secure secret>
#   OPENROUTER_API_KEY=<your key>
#   OPENAI_API_KEY=<your key>
#   DATABASE_URL=postgresql://user:pass@host:5432/ghost  # same DB as admin-api

# 3. Start Docker
docker compose up -d

# 4. Run migrations (one-time)
docker compose exec scribe-api sqlx migrate run

# 5. Verify scribe-api is running
curl http://localhost:8083/health
```

---

## 2. Nginx: Expose scribe-api

Create `/etc/nginx/sites-available/ghost-api`:

```nginx
server {
    listen 80;
    server_name api.ghost.exora.solutions;

    location / {
        proxy_pass http://127.0.0.1:8083;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket support (if scribe-api uses it)
        proxy_buffering off;
        proxy_read_timeout 86400;
    }
}
```

Enable and reload:

```bash
sudo ln -sf /etc/nginx/sites-available/ghost-api /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### HTTPS (Let's Encrypt)

```bash
sudo certbot --nginx -d api.ghost.exora.solutions
```

---

## 3. Build Scribe Installers

On a machine with Node, Rust, and platform build tools:

```bash
cd scribe/scribe

# Set production API URL
export APP_ENDPOINT=https://api.ghost.exora.solutions
export PAYMENT_ENDPOINT=https://api.ghost.exora.solutions
export API_ACCESS_KEY=<same as server .env>

# Build
./scripts/build-production.sh
```

Output: `src-tauri/target/release/bundle/`:
- **Windows**: `nsis/Ghost_*_setup.exe`, `msi/Ghost_*.msi`
- **macOS**: `dmg/Ghost_*.dmg`
- **Linux**: `deb/ghost_*.deb`, `appimage/Ghost_*.AppImage`

---

## 4. Host Installers on ghost.exora.solutions

Copy the built installers to the web UI's static folder so they are served at `/desktop/`:

```bash
# Create desktop folder
mkdir -p admin/admin-ui/public/desktop

# Copy built files (adjust paths to your build output)
cp scribe/scribe/src-tauri/target/release/bundle/nsis/Ghost_*_setup.exe admin/admin-ui/public/desktop/
cp scribe/scribe/src-tauri/target/release/bundle/msi/Ghost_*.msi admin/admin-ui/public/desktop/
cp scribe/scribe/src-tauri/target/release/bundle/dmg/Ghost_*.dmg admin/admin-ui/public/desktop/
cp scribe/scribe/src-tauri/target/release/bundle/deb/ghost_*.deb admin/admin-ui/public/desktop/
cp scribe/scribe/src-tauri/target/release/bundle/appimage/Ghost_*.AppImage admin/admin-ui/public/desktop/
```

Then rebuild and deploy the admin-ui. The download page at `/download` will link to these files.

**Alternative:** Host installers on GitHub Releases and update the download page links to point there instead of `/desktop/`.

---

## 5. Database: Shared with admin-api

scribe-api and admin-api must use the same PostgreSQL so licenses created via the web UI are visible to scribe-api. Set `DATABASE_URL` in `.env` to the production DB used by admin-api.

---

## 6. Environment Summary

| Variable | Where | Purpose |
|----------|-------|---------|
| `DATABASE_URL` | scribe-api .env | Same DB as admin-api |
| `API_ACCESS_KEY` | scribe-api .env, Scribe build | Must match |
| `PAYMENT_BASE_URL` | docker-compose.yml | https://ghost.exora.solutions |
| `OPENROUTER_API_KEY` | scribe-api .env | For chat |
| `OPENAI_API_KEY` | scribe-api .env | For Whisper STT |

---

## 7. User Flow

1. User visits https://ghost.exora.solutions/download
2. Clicks "Download for Windows" (or Mac/Linux)
3. Installs Scribe
4. Opens Ghost → connects to https://api.ghost.exora.solutions
5. Enters license or starts trial; checkout opens ghost.exora.solutions
6. Done
