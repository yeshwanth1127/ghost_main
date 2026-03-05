# Embedding model setup (intent router fast path)

If you set these up, the app can skip the LLM for simple requests (e.g. "write file X with content Y", "run npm install") and run them directly. If you don't, the app still works and every request goes to the LLM.

**You must build with the `embedding-router` feature** (see Step 4). The feature is off by default so the app builds on Windows without linker issues.

**Windows note:** The embedding-router feature (ort + tokenizers) can fail to link on Windows due to MSVC C++ runtime mismatch. If `npm run tauri:dev:embed` fails with a linker error (e.g. LNK2038), build in **WSL** or on **Linux/macOS**, or try the `load-dynamic` workaround in the Troubleshooting section.

---

## Step 1: Create a folder for the model

Create a folder where youâ€™ll put the ONNX file and tokenizer. For example:

- **Windows:** `C:\models\all-MiniLM-L6-v2`
- **Mac/Linux:** `~/models/all-MiniLM-L6-v2`

You can use any path you like; just remember it for the next steps.

---

## Step 2: Get the ONNX model and tokenizer from Hugging Face

### Option A: Download from the website (no Python)

1. Open: **https://huggingface.co/onnx-models/all-MiniLM-L6-v2-onnx**
2. Sign in or create a free Hugging Face account if needed.
3. Open the **Files and versions** tab.
4. Download:
   - **`model.onnx`** (the ONNX model)
   - **`tokenizer.json`** (the tokenizer)
5. Put both files in the folder you created in Step 1 (e.g. `C:\models\all-MiniLM-L6-v2\`).

So you should have:

- `C:\models\all-MiniLM-L6-v2\model.onnx`
- `C:\models\all-MiniLM-L6-v2\tokenizer.json`

### Option B: Download with Hugging Face CLI

1. Install the CLI (one time):
   ```bash
   pip install huggingface_hub
   ```
2. Log in (one time, need a Hugging Face token from https://huggingface.co/settings/tokens):
   ```bash
   huggingface-cli login
   ```
3. Download the repo into your folder (replace with your path):
   ```bash
   huggingface-cli download onnx-models/all-MiniLM-L6-v2-onnx --local-dir C:\models\all-MiniLM-L6-v2
   ```
4. In that folder you should have `model.onnx` and `tokenizer.json`. If the repo uses different names (e.g. `onnx/model.onnx`), move or copy the ONNX file and `tokenizer.json` to the same folder and use that path in Step 3.

### Option C: Export the model yourself with Python

1. Install:
   ```bash
   pip install "optimum[onnxruntime]"
   ```
2. Export:
   ```bash
   optimum-cli export onnx --model sentence-transformers/all-MiniLM-L6-v2 ./all-MiniLM-L6-v2-onnx/
   ```
3. In the created folder youâ€™ll get an ONNX file and `tokenizer.json`. Move that folder (or the two files) to something like `C:\models\all-MiniLM-L6-v2` and use that path in Step 3.

---

## Step 3: Tell the app where the files are (env vars)

The app reads these from a **`.env`** file (or from the environment). Use the **same directory as your Tauri app** so it finds the file when you run `npm run tauri dev`.

**Which `.env` to use**

- Use the one next to the Tauri app: **`scribe/scribe/src-tauri/.env`**
- If you run from the inner `scribe` folder, the app also checks **`scribe/scribe/.env`** and **`scribe/scribe/src-tauri/.env`**. So either is fine; **`src-tauri/.env`** is the one next to `env.example`.

**What to put in `.env`**

1. Open **`scribe/scribe/src-tauri/.env`** (create it if it doesnâ€™t exist; you can copy from **`scribe/scribe/src-tauri/env.example`**).
2. Add these two lines, with **your actual paths** (no quotes, no spaces around `=`):

   **Windows (example):**
   ```env
   EMBEDDING_MODEL_PATH=C:\models\all-MiniLM-L6-v2\model.onnx
   EMBEDDING_TOKENIZER_PATH=C:\models\all-MiniLM-L6-v2\tokenizer.json
   ```

   **Mac/Linux (example):**
   ```env
   EMBEDDING_MODEL_PATH=/home/you/models/all-MiniLM-L6-v2/model.onnx
   EMBEDDING_TOKENIZER_PATH=/home/you/models/all-MiniLM-L6-v2/tokenizer.json
   ```

3. Save the file.

**Important**

- Use the **full path** to each file.
- Paths must point to real files. If the paths are wrong or the files are missing, the router stays off and everything goes to the LLM (no crash).

---

## Step 3b: Windows only â€“ ONNX Runtime DLL (for embed build)

On Windows the embed build uses **load-dynamic** linking: the app loads `onnxruntime.dll` at runtime. You must set **`ORT_DYLIB_PATH`** in your `.env` to the **full path to `onnxruntime.dll`**.

1. **Download ONNX Runtime for Windows**
   - Go to [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases).
   - Download the **onnxruntime-win-x64-*.zip** (or win-x86 if 32-bit). Pick a recent release (e.g. 1.20.x).
   - Extract the zip to a folder (e.g. `C:\ysw\models\onnxruntime`).

2. **Set ORT_DYLIB_PATH in `.env`**
   - After extracting, you should have a file like `onnxruntime.dll` in the folder (or inside a subfolder like `bin` â€“ use the path to that DLL).
   - Add to **`scribe/scribe/src-tauri/.env`** (use your actual path, no quotes):
   ```env
   ORT_DYLIB_PATH=C:\ysw\models\onnxruntime\onnxruntime.dll
   ```
   - If the DLL is in a `bin` subfolder: `ORT_DYLIB_PATH=C:\ysw\models\onnxruntime\bin\onnxruntime.dll`

Without `ORT_DYLIB_PATH` set correctly, the embedding model will not load on Windows and every goal will go to the LLM (youâ€™ll see **embedding_unavailable** in the Router tab).

---

## Step 4: Build and run with the embedding-router feature

The embedding router is **not** included in the default build. You must pass the feature when building.

From your usual app folder (e.g. `scribe/scribe`):

**Development:**
```bash
npm run tauri:dev:embed
```

**Production build:**
```bash
npm run tauri:build:embed
```

If you prefer to call Tauri directly:
```bash
npx tauri dev -- --features embedding-router
npx tauri build -- --features embedding-router
```

On the first user message that hits the router, the app will load the ONNX model and tokenizer and build intent centroids. After that, simple requests can take the fast path without calling the LLM.

---

## Quick checklist

| Step | What you did |
|------|-------------------------------|
| 1    | Created a folder (e.g. `C:\models\all-MiniLM-L6-v2`) |
| 2    | Downloaded `model.onnx` and `tokenizer.json` from Hugging Face into that folder |
| 3    | Opened **`scribe/scribe/src-tauri/.env`** and added `EMBEDDING_MODEL_PATH=...` and `EMBEDDING_TOKENIZER_PATH=...` with full paths |
| 3b   | **Windows only:** Downloaded ONNX Runtime (onnxruntime-win-x64-*.zip), extracted it, and set `ORT_DYLIB_PATH=...` to the full path of `onnxruntime.dll` in `.env` |
| 4    | Ran **`npm run tauri:dev:embed`** (not plain `npm run tauri dev`) |

---

## Troubleshooting

- **Router not used / everything still goes to LLM**  
  Check that:
  1. You are running **`npm run tauri:dev:embed`** (not `npm run tauri dev`). The default command does not include the embedding router.
  2. Both `EMBEDDING_MODEL_PATH` and `EMBEDDING_TOKENIZER_PATH` are set in **`scribe/scribe/src-tauri/.env`** with full paths.
  3. **Windows:** `ORT_DYLIB_PATH` is set in `.env` to the **full path to `onnxruntime.dll`** (see Step 3b). Without it, the embedder cannot load.
  4. Paths are absolute and point to real files (open the path in Explorer/Finder to confirm).
  5. The app actually loads that `.env` (you should see something like "Loaded .env from: ... when the app starts).

- **Windows: linker error (LNK2038 or RuntimeLibrary mismatch)**  
  The `ort` crate can pull in a different C++ runtime than the rest of the app. Options:
  1. **Build in WSL or Linux/macOS** â€“ run `npm run tauri:dev:embed` from WSL or a Linux/macOS machine; the fast path will work there.
  2. **Try load-dynamic** â€“ in `src-tauri/Cargo.toml`, change the ort dependency to use the `load-dynamic` feature (see [ort linking docs](https://ort.pyke.io/setup/linking)). Then point `ORT_DYLIB_PATH` at a prebuilt ONNX Runtime DLL at runtime.

- **Different file names**  
  If the Hugging Face repo has a different layout (e.g. `onnx/model.onnx`), use the path to the **actual** `model.onnx` and `tokenizer.json` in your `EMBEDDING_MODEL_PATH` and `EMBEDDING_TOKENIZER_PATH`.
