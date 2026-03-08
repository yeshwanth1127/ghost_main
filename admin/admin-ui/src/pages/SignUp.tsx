import { useState } from "react";
import { Link } from "react-router-dom";
import { register, customerLogin } from "../api";

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
      <div className="flex min-h-[calc(100vh-4rem)] items-center justify-center px-4 py-12">
        <div className="w-full max-w-sm rounded-lg border border-ghost-border bg-ghost-surface p-8 shadow-lg">
          <h1 className="mb-4 text-2xl font-semibold text-ghost-text">Account created</h1>
          <p className="mb-4 text-sm text-ghost-muted">
            Your 14-day free trial has started. Use this license key in the Ghost app:
          </p>
          <code className="mb-4 block break-all rounded-md bg-ghost-bg p-4 text-sm text-ghost-text">
            {success.license_key}
          </code>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => navigator.clipboard.writeText(success.license_key)}
              className="rounded-md bg-ghost-border px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-ghost-muted"
            >
              Copy
            </button>
            <Link
              to="/subscriptions"
              className="rounded-md bg-ghost-accent px-4 py-2 text-sm font-medium text-white no-underline transition-colors hover:bg-blue-600"
            >
              Upgrade plan
            </Link>
            <Link
              to="/login"
              className="rounded-md border border-ghost-border bg-transparent px-4 py-2 text-sm font-medium text-ghost-muted no-underline transition-colors hover:text-ghost-text"
            >
              Sign in
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-4rem)] items-center justify-center px-4 py-12">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm rounded-lg border border-ghost-border bg-ghost-surface p-8 shadow-lg"
      >
        <h1 className="mb-6 text-2xl font-semibold text-ghost-text">Sign up</h1>
        {error && (
          <div className="mb-4 text-sm text-ghost-error">{error}</div>
        )}
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
        <div className="mb-6">
          <label className="mb-2 block text-sm text-ghost-muted">Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
            className="w-full rounded-md border border-ghost-border bg-ghost-bg px-3 py-2 text-ghost-text placeholder-ghost-muted focus:border-ghost-accent focus:outline-none focus:ring-1 focus:ring-ghost-accent"
          />
        </div>
        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-md bg-ghost-accent px-4 py-3 font-medium text-white transition-colors hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-70"
        >
          {loading ? "Creating account..." : "Sign up"}
        </button>
        <p className="mt-4 text-center text-sm text-ghost-muted">
          Already have an account?{" "}
          <Link to="/login" className="text-ghost-accent hover:underline">
            Sign in
          </Link>
        </p>
      </form>
    </div>
  );
}
