import { useState } from "react";
import { useNavigate, Link } from "react-router-dom";
import { login, customerLogin } from "../api";

const formStyle = {
  padding: "2rem",
  background: "#1e293b",
  borderRadius: "8px",
  width: "320px",
  boxShadow: "0 4px 6px rgba(0,0,0,0.3)",
};

const inputStyle = {
  width: "100%",
  padding: "0.5rem 0.75rem",
  background: "#0f172a",
  border: "1px solid #334155",
  borderRadius: "4px",
  color: "#e2e8f0",
  fontSize: "1rem",
};

type Mode = "admin" | "customer";

export default function Login() {
  const [mode, setMode] = useState<Mode>("customer");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      if (mode === "admin") {
        const { token } = await login(username, password);
        localStorage.setItem("admin_token", token);
        localStorage.removeItem("customer_token");
        localStorage.removeItem("customer_email");
        localStorage.removeItem("customer_license");
        navigate("/dashboard", { replace: true });
      } else {
        const data = await customerLogin(email, password);
        localStorage.setItem("customer_token", data.token);
        localStorage.setItem("customer_email", data.email);
        localStorage.setItem("customer_license", data.license_key);
        localStorage.setItem("customer_user_id", data.user_id);
        localStorage.removeItem("admin_token");
        navigate("/account", { replace: true });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: "flex", minHeight: "100vh", alignItems: "center", justifyContent: "center" }}>
      <form onSubmit={handleSubmit} style={formStyle}>
        <h1 style={{ margin: "0 0 1.5rem", fontSize: "1.5rem" }}>Ghost</h1>
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
          <button
            type="button"
            onClick={() => setMode("customer")}
            style={{
              flex: 1,
              padding: "0.5rem",
              background: mode === "customer" ? "#334155" : "transparent",
              border: "1px solid #334155",
              borderRadius: "4px",
              color: "#e2e8f0",
              cursor: "pointer",
            }}
          >
            Customer
          </button>
          <button
            type="button"
            onClick={() => setMode("admin")}
            style={{
              flex: 1,
              padding: "0.5rem",
              background: mode === "admin" ? "#334155" : "transparent",
              border: "1px solid #334155",
              borderRadius: "4px",
              color: "#e2e8f0",
              cursor: "pointer",
            }}
          >
            Admin
          </button>
        </div>
        {error && (
          <div style={{ color: "#f87171", marginBottom: "1rem", fontSize: "0.875rem" }}>{error}</div>
        )}
        {mode === "admin" ? (
          <div style={{ marginBottom: "1rem" }}>
            <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              autoComplete="username"
              style={inputStyle}
            />
          </div>
        ) : (
          <div style={{ marginBottom: "1rem" }}>
            <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              autoComplete="email"
              style={inputStyle}
            />
          </div>
        )}
        <div style={{ marginBottom: "1.5rem" }}>
          <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            autoComplete={mode === "admin" ? "current-password" : "current-password"}
            style={inputStyle}
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
        {mode === "customer" && (
          <p style={{ marginTop: "1rem", fontSize: "0.875rem", color: "#94a3b8" }}>
            Don&apos;t have an account? <Link to="/signup" style={{ color: "#3b82f6" }}>Sign up</Link>
          </p>
        )}
      </form>
    </div>
  );
}
