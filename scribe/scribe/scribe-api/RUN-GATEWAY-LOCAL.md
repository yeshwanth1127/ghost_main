# Running ghost-api with gateway (agent mode)

Something else is already using port 8081 — that process is an **old** ghost-api build (no gateway). So your new `cargo run` either never binds, or you're starting the wrong binary.

## Option 1: Run on port 8082 (no conflict)

1. **In `scribe-api` folder**, create or edit `.env` and set:
   ```env
   PORT=8082
   ```
   (Keep your existing DATABASE_URL, etc.)

2. **Stop any other ghost-api** (optional, so only one server runs):
   - If you use PM2: `pm2 stop ghost-api`
   - Or close the terminal where an old `cargo run` is still running
   - Or in Task Manager, end any `ghost-api.exe` or similar

3. **Start the API** from the scribe-api folder:
   ```powershell
   cd c:\ysw\ghost\ghost_main\scribe\scribe\scribe-api
   cargo run
   ```

4. **You must see** in the terminal:
   ```text
   >>> ghost-api starting (with GATEWAY) <<<
   ...
   - GET  http://localhost:8082/gateway-ping (gateway check)
   - WS   ws://localhost:8082/gateway (agent mode)
   ```
   If you don't see `(with GATEWAY)` and the gateway-ping line, the wrong binary is running.

5. **Check in the browser:**
   - http://127.0.0.1:8082/health → should include `"gateway": true`
   - http://127.0.0.1:8082/gateway-ping → `{"ok":true,"gateway":"ready"}`

6. **Point the Ghost app at 8082:**  
   In the **frontend** `.env` (scribe/scribe/.env) set:
   ```env
   VITE_GHOST_GATEWAY_WS_URL=ws://127.0.0.1:8082/gateway
   ```
   Restart the Ghost app and connect again.

## Option 2: Use port 8081 (replace the old process)

1. **Stop whatever is on 8081:**
   - PM2: `pm2 stop ghost-api` (or `pm2 delete ghost-api`)
   - Or close the terminal with the old `cargo run`
   - Or in Task Manager, end the process using port 8081

2. **In scribe-api `.env`** set `PORT=8081` (or leave as 8081 if that's what you use).

3. **From scribe-api folder:**
   ```powershell
   cargo run
   ```
   You must see `>>> ghost-api starting (with GATEWAY) <<<`. If you see "FATAL: Could not bind", something is still using 8081.

4. Test http://127.0.0.1:8081/health (should include `"gateway": true`) and http://127.0.0.1:8081/gateway-ping.

## If `cargo run` says "Could not bind"

Another process is using the port. Stop it (PM2, other terminal, Task Manager) or use PORT=8082 as in Option 1.
