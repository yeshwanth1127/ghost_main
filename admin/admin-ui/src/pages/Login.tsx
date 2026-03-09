import { useState } from "react";
import { useNavigate, Link } from "react-router-dom";
import { customerLogin, sendLoginOtp, verifyTrialOtp } from "../api";

type CustomerAuthMode = "password" | "otp";

export default function Login() {
  const [customerAuthMode, setCustomerAuthMode] = useState<CustomerAuthMode>("otp");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [otp, setOtp] = useState("");
  const [otpSent, setOtpSent] = useState(false);
  const [resendCooldown, setResendCooldown] = useState(0);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      if (customerAuthMode === "password") {
        const data = await customerLogin(email, password);
        localStorage.setItem("customer_token", data.token);
        localStorage.setItem("customer_email", data.email);
        localStorage.setItem("customer_license", data.license_key);
        localStorage.setItem("customer_user_id", data.user_id);
        if (data.plan) localStorage.setItem("customer_plan", data.plan);
        localStorage.removeItem("admin_token");
        navigate("/account", { replace: true });
      } else {
        const data = await verifyTrialOtp(email, otp);
        if (data.token && data.email && data.license_key) {
          localStorage.setItem("customer_token", data.token);
          localStorage.setItem("customer_email", data.email);
          localStorage.setItem("customer_license", data.license_key);
          if (data.user_id) localStorage.setItem("customer_user_id", data.user_id);
          if (data.plan) localStorage.setItem("customer_plan", data.plan);
          localStorage.removeItem("admin_token");
          navigate("/account", { replace: true });
        } else {
          setError(data.message || "Verification failed");
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  };

  const handleSendOtp = async () => {
    const em = email.trim();
    if (!em || !em.includes("@")) {
      setError("Please enter a valid email");
      return;
    }
    setError("");
    setLoading(true);
    try {
      await sendLoginOtp(em);
      setOtpSent(true);
      setOtp("");
      setResendCooldown(60);
      const interval = setInterval(() => {
        setResendCooldown((s) => {
          if (s <= 1) {
            clearInterval(interval);
            return 0;
          }
          return s - 1;
        });
      }, 1000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to send code");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm rounded-lg border border-white/20 bg-white/5 p-6 sm:p-8 mx-4"
        style={{ fontFamily: "Space Grotesk, sans-serif" }}
      >
        <h1 className="mb-6 text-xl sm:text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>
          Ghost
        </h1>
        {error && (
          <div className="mb-4 text-sm text-red-400">{error}</div>
        )}
        <div className="mb-4 flex gap-2">
          <button
            type="button"
            onClick={() => { setCustomerAuthMode("otp"); setOtpSent(false); setOtp(""); setError(""); }}
            className={`flex-1 rounded-lg border px-4 py-2 text-sm font-medium transition-colors ${
              customerAuthMode === "otp"
                ? "border-white/50 bg-white/10 text-white"
                : "border-white/20 bg-transparent text-white/70 hover:text-white"
            }`}
          >
            Email + OTP
          </button>
          <button
            type="button"
            onClick={() => { setCustomerAuthMode("password"); setOtpSent(false); setError(""); }}
            className={`flex-1 rounded-lg border px-4 py-2 text-sm font-medium transition-colors ${
              customerAuthMode === "password"
                ? "border-white/50 bg-white/10 text-white"
                : "border-white/20 bg-transparent text-white/70 hover:text-white"
            }`}
          >
            Password
          </button>
        </div>
        <div className="mb-4">
          <label className="mb-2 block text-sm" style={{ color: "#c96a5b" }}>Email</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            autoComplete="email"
            disabled={otpSent}
            className="w-full rounded-lg border border-white/30 bg-black px-3 py-2 text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b] disabled:opacity-70"
          />
        </div>
        {customerAuthMode === "otp" ? (
          <>
            {!otpSent ? (
              <button
                type="button"
                onClick={handleSendOtp}
                disabled={loading}
                className="mb-6 w-full rounded-lg border border-white bg-black px-4 py-3 font-medium text-white transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-70"
              >
                {loading ? "Sending..." : "Send verification code"}
              </button>
            ) : (
              <>
                <div className="mb-4">
                  <label className="mb-2 block text-sm" style={{ color: "#c96a5b" }}>Verification code</label>
                  <input
                    type="text"
                    inputMode="numeric"
                    pattern="[0-9]*"
                    maxLength={6}
                    value={otp}
                    onChange={(e) => setOtp(e.target.value.replace(/\D/g, ""))}
                    placeholder="000000"
                    required
                    className="w-full rounded-lg border border-white/30 bg-black px-3 py-2.5 text-center text-lg tracking-[0.5em] text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
                  />
                </div>
                <div className="mb-6 flex gap-2">
                  <button
                    type="submit"
                    disabled={loading}
                    className="flex-1 rounded-lg border border-white bg-white px-4 py-3 font-medium text-black transition-colors hover:bg-white/90 disabled:cursor-not-allowed disabled:opacity-70"
                  >
                    {loading ? "Signing in..." : "Sign in"}
                  </button>
                  <button
                    type="button"
                    onClick={() => setOtpSent(false)}
                    disabled={loading}
                    className="rounded-lg border border-white/30 px-4 py-3 font-medium text-white hover:bg-white/10"
                  >
                    Back
                  </button>
                </div>
                <button
                  type="button"
                  onClick={handleSendOtp}
                  disabled={loading || resendCooldown > 0}
                  className="w-full text-sm text-[#ff9a8b] hover:underline disabled:opacity-50 disabled:cursor-not-allowed disabled:no-underline"
                >
                  {resendCooldown > 0 ? `Resend code (${resendCooldown}s)` : "Resend code"}
                </button>
              </>
            )}
          </>
        ) : (
          <div className="mb-6">
            <label className="mb-2 block text-sm" style={{ color: "#c96a5b" }}>Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              autoComplete="current-password"
              className="w-full rounded-lg border border-white/30 bg-black px-3 py-2 text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
            />
          </div>
        )}
        {customerAuthMode === "password" && (
          <button
            type="submit"
            disabled={loading}
            className="w-full rounded-lg border border-white bg-black px-4 py-3 font-medium text-white transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-70"
          >
            {loading ? "Signing in..." : "Sign in"}
          </button>
        )}
        <p className="mt-4 text-center text-sm text-white/80">
          Don&apos;t have an account?{" "}
          <Link to="/subscriptions" className="text-[#ff9a8b] hover:underline">
            Start free trial
          </Link>
        </p>
        <p className="mt-2 text-center text-xs text-white/50">
          <Link to="/admin-login" className="hover:underline">
            Admin login
          </Link>
        </p>
      </form>
    </div>
  );
}
