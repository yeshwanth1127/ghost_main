import { useState } from "react";
import { useNavigate, Link } from "react-router-dom";
import { login, customerLogin } from "../api";

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
    <div className="flex min-h-[calc(100vh-4rem)] items-center justify-center px-4 py-12">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm rounded-lg border border-ghost-border bg-ghost-surface p-8 shadow-lg"
      >
        <h1 className="mb-6 text-2xl font-semibold text-ghost-text">Ghost</h1>
        <div className="mb-4 flex gap-2">
          <button
            type="button"
            onClick={() => setMode("customer")}
            className={`flex-1 rounded-md border px-4 py-2 text-sm font-medium transition-colors ${
              mode === "customer"
                ? "border-ghost-border bg-ghost-border/50 text-ghost-text"
                : "border-ghost-border bg-transparent text-ghost-muted hover:text-ghost-text"
            }`}
          >
            Customer
          </button>
          <button
            type="button"
            onClick={() => setMode("admin")}
            className={`flex-1 rounded-md border px-4 py-2 text-sm font-medium transition-colors ${
              mode === "admin"
                ? "border-ghost-border bg-ghost-border/50 text-ghost-text"
                : "border-ghost-border bg-transparent text-ghost-muted hover:text-ghost-text"
            }`}
          >
            Admin
          </button>
        </div>
        {error && (
          <div className="mb-4 text-sm text-ghost-error">{error}</div>
        )}
        {mode === "admin" ? (
          <div className="mb-4">
            <label className="mb-2 block text-sm text-ghost-muted">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              autoComplete="username"
              className="w-full rounded-md border border-ghost-border bg-ghost-bg px-3 py-2 text-ghost-text placeholder-ghost-muted focus:border-ghost-accent focus:outline-none focus:ring-1 focus:ring-ghost-accent"
            />
          </div>
        ) : (
          <div className="mb-4">
            <label className="mb-2 block text-sm text-ghost-muted">Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              autoComplete="email"
              className="w-full rounded-md border border-ghost-border bg-ghost-bg px-3 py-2 text-ghost-text placeholder-ghost-muted focus:border-ghost-accent focus:outline-none focus:ring-1 focus:ring-ghost-accent"
            />
          </div>
        )}
        <div className="mb-6">
          <label className="mb-2 block text-sm text-ghost-muted">Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            autoComplete="current-password"
            className="w-full rounded-md border border-ghost-border bg-ghost-bg px-3 py-2 text-ghost-text placeholder-ghost-muted focus:border-ghost-accent focus:outline-none focus:ring-1 focus:ring-ghost-accent"
          />
        </div>
        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-md bg-ghost-accent px-4 py-3 font-medium text-white transition-colors hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-70"
        >
          {loading ? "Signing in..." : "Sign in"}
        </button>
        {mode === "customer" && (
          <p className="mt-4 text-center text-sm text-ghost-muted">
            Don&apos;t have an account?{" "}
            <Link to="/signup" className="text-ghost-accent hover:underline">
              Sign up
            </Link>
          </p>
        )}
      </form>
    </div>
  );
}
