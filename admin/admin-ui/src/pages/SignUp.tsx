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
      if (loginData.plan) localStorage.setItem("customer_plan", loginData.plan);
      setSuccess({ license_key: data.license_key });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Registration failed");
    } finally {
      setLoading(false);
    }
  };

  if (success) {
    return (
      <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32">
        <div className="w-full max-w-sm rounded-lg border border-white/20 bg-white/5 p-6 sm:p-8 mx-4" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
          <h1 className="mb-4 text-xl sm:text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>Account created</h1>
          <p className="mb-4 text-sm text-white">
            Your 14-day free trial has started. Use this license key in the Ghost app:
          </p>
          <code className="mb-4 block break-all rounded-lg bg-black p-4 text-sm text-white">
            {success.license_key}
          </code>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => navigator.clipboard.writeText(success.license_key)}
              className="rounded-lg border border-white px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-white/10"
            >
              Copy
            </button>
            <Link
              to="/subscriptions"
              className="rounded-lg border border-white bg-black px-4 py-2 text-sm font-medium text-white no-underline transition-colors hover:bg-white/10"
            >
              Upgrade plan
            </Link>
            <Link
              to="/login"
              className="rounded-lg border border-white/30 bg-transparent px-4 py-2 text-sm font-medium text-white/80 no-underline transition-colors hover:text-white"
            >
              Sign in
            </Link>
          </div>
        </div>
      </div>
    );
  }


  return (
    <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm rounded-lg border border-white/20 bg-white/5 p-6 sm:p-8 mx-4"
        style={{ fontFamily: "Space Grotesk, sans-serif" }}
      >
        <h1 className="mb-6 text-xl sm:text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>Sign up</h1>
        {error && (
          <div className="mb-4 text-sm text-red-400">{error}</div>
        )}
        <div className="mb-4">
          <label className="mb-2 block text-sm" style={{ color: "#c96a5b" }}>Email</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            autoComplete="email"
            className="w-full rounded-lg border border-white/30 bg-black px-3 py-2 text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
          />
        </div>
        <div className="mb-6">
          <label className="mb-2 block text-sm" style={{ color: "#c96a5b" }}>Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={6}
            autoComplete="new-password"
            className="w-full rounded-lg border border-white/30 bg-black px-3 py-2 text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
          />
        </div>
        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-lg border border-white bg-black px-4 py-3 font-medium text-white transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-70"
        >
          {loading ? "Creating account..." : "Sign up"}
        </button>
        <p className="mt-4 text-center text-sm text-white/80">
          Already have an account?{" "}
          <Link to="/login" className="text-[#ff9a8b] hover:underline">
            Sign in
          </Link>
        </p>
      </form>
    </div>
  );
}
