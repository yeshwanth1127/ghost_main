"""
Local embeddings service (dev): sentence-transformers/all-MiniLM-L6-v2.
GPU if available, else CPU. FastAPI, POST /embed. No Docker, no vector DB.
"""
import torch
from fastapi import FastAPI
from pydantic import BaseModel

from sentence_transformers import SentenceTransformer

app = FastAPI(title="Embeddings (dev)")

# Load once at startup; device auto-detect
device = "cuda" if torch.cuda.is_available() else "cpu"
model = None

BATCH_SIZE = 8


@app.on_event("startup")
def load_model():
    global model
    model = SentenceTransformer("sentence-transformers/all-MiniLM-L6-v2", device=device)


class EmbedRequest(BaseModel):
    texts: list[str]


class EmbedResponse(BaseModel):
    embeddings: list[list[float]]
    device: str


@app.post("/embed", response_model=EmbedResponse)
def embed(request: EmbedRequest):
    if not request.texts:
        return EmbedResponse(embeddings=[], device=device)
    vecs = model.encode(
        request.texts,
        batch_size=BATCH_SIZE,
        normalize_embeddings=True,
        convert_to_numpy=True,
    )
    if vecs.ndim == 1:
        vecs = vecs.reshape(1, -1)
    embeddings = [vecs[i].tolist() for i in range(len(vecs))]
    return EmbedResponse(embeddings=embeddings, device=device)


@app.get("/health")
def health():
    return {"device": device, "model_loaded": model is not None}
