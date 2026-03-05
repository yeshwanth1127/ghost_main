import { useState } from "react";
import { Link } from "react-router-dom";
import { register, customerLogin } from "../api";

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

export default function SignUp() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState<{ license_key: string } | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const data = await register(email, password);
      const loginData = await customerLogin(email, password);
      localStorage.setItem("customer_token", loginData.token);
      localStorage.setItem("customer_email", loginData.email);
      localStorage.setItem("customer_license", loginData.license_key);
      localStorage.setItem("customer_user_id", loginData.user_id);
      setSuccess({ license_key: data.license_key });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Registration failed");
    } finally {
      setLoading(false);
    }
  };

  if (success) {
    return (
      <div style={{ display: "flex", minHeight: "100vh", alignItems: "center", justifyContent: "center" }}>
        <div style={formStyle}>
          <h1 style={{ margin: "0 0 1rem", fontSize: "1.5rem" }}>Account created</h1>
          <p style={{ color: "#94a3b8", marginBottom: "1rem", fontSize: "0.875rem" }}>
            Your 14-day free trial has started. Use this license key in the Ghost app:
          </p>
          <code
            style={{
              display: "block",
              padding: "1rem",
              background: "#0f172a",
              borderRadius: "4px",
              marginBottom: "1rem",
              wordBreak: "break-all",
            }}
          >
            {success.license_key}
          </code>
          <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
            <button
              onClick={() => navigator.clipboard.writeText(success.license_key)}
              style={{
                padding: "0.5rem 1rem",
                background: "#334155",
                border: "none",
                borderRadius: "4px",
                color: "white",
                cursor: "pointer",
              }}
            >
              Copy
            </button>
            <Link
              to="/subscriptions"
              style={{
                padding: "0.5rem 1rem",
                background: "#3b82f6",
                borderRadius: "4px",
                color: "white",
                textDecoration: "none",
              }}
            >
              Upgrade plan
            </Link>
            <Link
              to="/login"
              style={{
                padding: "0.5rem 1rem",
                background: "transparent",
                borderRadius: "4px",
                color: "#94a3b8",
                textDecoration: "none",
                border: "1px solid #334155",
              }}
            >
              Sign in
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", minHeight: "100vh", alignItems: "center", justifyContent: "center" }}>
      <form onSubmit={handleSubmit} style={formStyle}>
        <h1 style={{ margin: "0 0 1.5rem", fontSize: "1.5rem" }}>Sign up</h1>
        {error && (
          <div style={{ color: "#f87171", marginBottom: "1rem", fontSize: "0.875rem" }}>{error}</div>
        )}
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
        <div style={{ marginBottom: "1.5rem" }}>
          <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
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
          {loading ? "Creating account..." : "Sign up"}
        </button>
        <p style={{ marginTop: "1rem", fontSize: "0.875rem", color: "#94a3b8" }}>
          Already have an account? <Link to="/login" style={{ color: "#3b82f6" }}>Sign in</Link>
        </p>
      </form>
    </div>
  );
}
