# Local Embeddings Service (Dev)

Python service: `sentence-transformers/all-MiniLM-L6-v2`, GPU if available else CPU.  
Rust (Tauri) talks to it via HTTP only. No Docker, no vector DB.

---

## STEP 1 — GPU verification (mandatory)

**Option A: Run .bat scripts (from `services\embeddings`)**
- `step1_nvidia_smi.bat` — verify NVIDIA driver only
- `step1_venv_and_torch.bat` — create venv, install CUDA PyTorch, run `verify_gpu.py`
- `step1_full.bat` — do everything above in one go

**Option B: Manual commands**

**1. Verify NVIDIA driver**
```cmd
nvidia-smi
```

**2. Create venv and activate (Windows)**
```cmd
cd services\embeddings
python -m venv ai-env
ai-env\Scripts\activate
```

**3. Install CUDA-enabled PyTorch** (torch only; torchvision/torchaudio not needed for embeddings and may lack wheels on some Python versions)
```cmd
pip install torch --index-url https://download.pytorch.org/whl/cu118
```

**4. Run GPU check script**
```cmd
pip install torch
python verify_gpu.py
```
Expected: prints `torch.cuda.is_available(): True` and GPU name, or `False` and "N/A" on CPU.

---

## STEP 2 — Hugging Face authentication

**1. Create a token**  
[Hugging Face → Settings → Access Tokens](https://huggingface.co/settings/tokens). Create a **Read** token.

**2. Install Hub and login**
```cmd
pip install huggingface_hub
huggingface-cli login
```
Paste token when prompted. Do **not** hardcode it in code.

**3. Where the token is stored (Windows)**  
`%USERPROFILE%\.cache\huggingface\token` (or `huggingface-cli whoami` to confirm login).

**4. Optional env var (for CI/scripts)**  
```cmd
setx HF_TOKEN "hf_xxx"
```
Use `HF_TOKEN` in scripts only; do not commit. Prefer `huggingface-cli login` for local dev.

---

## STEP 3 — Directory structure

```
services/
  embeddings/
    app.py
    requirements.txt
    verify_gpu.py
    README.md
```

---

## STEP 4 & 5 — App and dependencies

- **app.py**: FastAPI, model loaded once at startup, device = `cuda` if `torch.cuda.is_available()` else `cpu`.
- **POST /embed**: body `{ "texts": ["string", ...] }` → `{ "embeddings": [[...], ...], "device": "cuda"|"cpu" }`. Normalized, batch size 8.
- **requirements.txt**: torch, sentence-transformers, fastapi, uvicorn, huggingface_hub.

---

## STEP 6 — How to run (dev)

**Install and run**
```cmd
cd services\embeddings
ai-env\Scripts\activate
pip install -r requirements.txt
uvicorn app:app --reload --port 8004
```

**Test with curl**
```cmd
curl -X POST http://localhost:8004/embed -H "Content-Type: application/json" -d "{\"texts\": [\"hello world\"]}"
```
Expected: JSON with `embeddings` (list of 384-dim vectors) and `device`.

---

## STEP 7 — Rust access (overview)

- **No PyO3, no Python in Rust.** Rust talks to the Python service over HTTP only.
- **Endpoint**: `POST http://localhost:8004/embed` (or set base URL via env, e.g. `EMBEDDING_SERVICE_URL`).
- **Request**: JSON body `{ "texts": ["s1", "s2", ...] }`.
- **Response**: `{ "embeddings": [[f32, ...], ...], "device": "cuda"|"cpu" }`.
- **Rust side**: use `reqwest` (or Tauri’s HTTP plugin) to POST the list of strings, parse JSON into `Vec<Vec<f32>>`. Same code works whether Python runs on GPU or CPU; device is informational only.
- **Integration**: In the Tauri embedding module, add an optional “HTTP embedder” backend that calls this service when `EMBEDDING_SERVICE_URL` is set (fallback after ONNX and API providers). No Python process embedding; no async queues or workers.
