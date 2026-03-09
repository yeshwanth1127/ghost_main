# Deploying Ghost Admin to ghost.exora.solutions

This guide deploys the admin-ui and admin-api on **ghost.exora.solutions** using the same domain. The API is served at `/api`.

## Architecture

```
ghost.exora.solutions/          → admin-ui (static files)
ghost.exora.solutions/api/*     → admin-api (Rust backend, port 6660)
```

**Razorpay webhook URL:** `https://ghost.exora.solutions/api/payments/webhook`

---

## Step 1: Build the frontend

```bash
cd ghost_main/admin/admin-ui
npm install
npm run build
```

Output: `admin-ui/dist/` (static HTML, JS, CSS)

---

## Step 2: Build the admin-api (on your server)

On your VPS/server:

```bash
cd ghost_main/admin/admin-api
cargo build --release
```

Binary: `target/release/admin_api.exe` (Windows) or `target/release/admin_api` (Linux)

---

## Step 3: Set up environment on server

Create `admin-api/.env` on the server with:

```env
DATABASE_URL=postgres://user:pass@host:5432/dbname
ADMIN_PORT=6660

# Razorpay
RAZORPAY_KEY_ID=rzp_live_xxxx
RAZORPAY_KEY_SECRET=xxxx
RAZORPAY_PLAN_STARTER=plan_xxxx
RAZORPAY_PLAN_PRO=plan_xxxx
RAZORPAY_PLAN_POWER=plan_xxxx
RAZORPAY_WEBHOOK_SECRET=xxxx

# SMTP (Hostinger)
# Email for OTP verification - use ONE of:

# Option A: Resend (recommended - works from any server, free 100/day)
# 1. Sign up at resend.com
# 2. Add domain exora.solutions, add DNS records (SPF, DKIM)
# 3. Create API key, add below
RESEND_API_KEY=re_xxxxxxxxxxxx

# Option B: Hostinger SMTP (may be blocked from external VPS)
SMTP_HOST=smtp.hostinger.com
SMTP_PORT=587
SMTP_USERNAME=support@exora.solutions
SMTP_PASSWORD=your_password
SMTP_FROM_EMAIL=support@exora.solutions
# If 587 fails, try: SMTP_PORT=465
```

---

## Step 4: Run admin-api as a service

### Linux (systemd)

Create `/etc/systemd/system/ghost-admin-api.service`:

```ini
[Unit]
Description=Ghost Admin API
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/path/to/ghost_main/admin/admin-api
ExecStart=/path/to/ghost_main/admin/admin-api/target/release/admin_api
Restart=always
RestartSec=5
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable ghost-admin-api
sudo systemctl start ghost-admin-api
```

### Windows (as a service or PM2)

Run the binary and keep it running, or use NSSM to install as a Windows service.

---

## Step 5: Configure Nginx reverse proxy

Install Nginx, then create a config (e.g. `/etc/nginx/sites-available/ghost-admin`):

```nginx
server {
    listen 80;
    server_name ghost.exora.solutions;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name ghost.exora.solutions;

    # SSL certificates (Let's Encrypt or your provider)
    ssl_certificate /etc/letsencrypt/live/ghost.exora.solutions/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ghost.exora.solutions/privkey.pem;

    # Static files (admin-ui)
    root /path/to/ghost_main/admin/admin-ui/dist;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    # API proxy
    location /api/ {
        proxy_pass http://127.0.0.1:6660;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable and reload:

```bash
sudo ln -s /etc/nginx/sites-available/ghost-admin /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

---

## Step 6: SSL with Let's Encrypt (if not already)

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d ghost.exora.solutions
```

---

## Step 7: Razorpay webhook

1. Razorpay Dashboard → **Settings** → **Webhooks**
2. Add URL: `https://ghost.exora.solutions/api/payments/webhook`
3. Enable events: `subscription.charged`, `subscription.cancelled`, `subscription.completed`, `subscription.halted`, `subscription.activated`
4. Copy the **Webhook Secret** → add to `RAZORPAY_WEBHOOK_SECRET` in `.env`
5. Restart admin-api after updating `.env`

---

## Checklist

- [ ] admin-ui built (`npm run build`)
- [ ] admin-api built (`cargo build --release`)
- [ ] `.env` configured on server
- [ ] admin-api running (systemd or equivalent)
- [ ] Nginx serving static files and proxying `/api`
- [ ] SSL enabled (HTTPS)
- [ ] Razorpay webhook URL set to `https://ghost.exora.solutions/api/payments/webhook`
- [ ] DNS: `ghost.exora.solutions` points to your server IP

---

## Updating

1. Pull latest code
2. Rebuild admin-ui: `npm run build`
3. Rebuild admin-api: `cargo build --release`
4. Restart admin-api service
5. Copy new `dist/` to server if built elsewhere
