import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { login } from "../api";

export default function Login() {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const { token } = await login(username, password);
      localStorage.setItem("admin_token", token);
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: "flex", minHeight: "100vh", alignItems: "center", justifyContent: "center" }}>
      <form
        onSubmit={handleSubmit}
        style={{
          padding: "2rem",
          background: "#1e293b",
          borderRadius: "8px",
          width: "320px",
          boxShadow: "0 4px 6px rgba(0,0,0,0.3)",
        }}
      >
        <h1 style={{ margin: "0 0 1.5rem", fontSize: "1.5rem" }}>Ghost Admin</h1>
        {error && (
          <div style={{ color: "#f87171", marginBottom: "1rem", fontSize: "0.875rem" }}>
            {error}
          </div>
        )}
        <div style={{ marginBottom: "1rem" }}>
          <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>
            Username
          </label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            autoComplete="username"
            style={{
              width: "100%",
              padding: "0.5rem 0.75rem",
              background: "#0f172a",
              border: "1px solid #334155",
              borderRadius: "4px",
              color: "#e2e8f0",
              fontSize: "1rem",
            }}
          />
        </div>
        <div style={{ marginBottom: "1.5rem" }}>
          <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>
            Password
          </label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            autoComplete="current-password"
            style={{
              width: "100%",
              padding: "0.5rem 0.75rem",
              background: "#0f172a",
              border: "1px solid #334155",
              borderRadius: "4px",
              color: "#e2e8f0",
              fontSize: "1rem",
            }}
          />
        </div>
        <button
          type="submit"
          disabled={loading}
          style={{
            width: "100%",
            padding: "0.75rem",
            background: "#3b82f6",
            border: "none",
            borderRadius: "4px",
            color: "white",
            fontSize: "1rem",
            cursor: loading ? "not-allowed" : "pointer",
          }}
        >
          {loading ? "Signing in..." : "Sign in"}
        </button>
      </form>
    </div>
  );
}
