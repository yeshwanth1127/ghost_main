import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { sendTrialOtp, verifyTrialOtp, createSubscription } from "../api";

const PLANS = [
  { id: "starter", name: "Starter", price: "₹189", desc: "For light usage" },
  { id: "pro", name: "Pro", price: "₹279", desc: "For power users" },
  { id: "power", name: "Power", price: "₹599", desc: "Maximum capacity" },
];

type TrialModalStep = "email" | "otp" | "success";
type PayModalStep = "email" | "loading";

declare global {
  interface Window {
    Razorpay: new (options: {
      key: string;
      subscription_id: string;
      name?: string;
      description?: string;
      callback_url?: string;
      prefill?: { email?: string };
      theme?: { color?: string };
    }) => { open: () => void };
  }
}

export default function Subscriptions() {
  const navigate = useNavigate();
  const [trialModalOpen, setTrialModalOpen] = useState(false);
  const [trialStep, setTrialStep] = useState<TrialModalStep>("email");
  const [email, setEmail] = useState("");
  const [otp, setOtp] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [licenseKey, setLicenseKey] = useState("");
  const [verifiedEmail, setVerifiedEmail] = useState("");

  const [payModalOpen, setPayModalOpen] = useState(false);
  const [payStep, setPayStep] = useState<PayModalStep>("email");
  const [payEmail, setPayEmail] = useState("");
  const [selectedPlan, setSelectedPlan] = useState<typeof PLANS[0] | null>(null);

  const customerEmail = localStorage.getItem("customer_email");
  const customerUserId = localStorage.getItem("customer_user_id");
  const customerLicense = localStorage.getItem("customer_license");
  const isLoggedIn = !!localStorage.getItem("customer_token");

  const openTrialModal = () => {
    setTrialModalOpen(true);
    setTrialStep("email");
    setEmail("");
    setOtp("");
    setError("");
    setLicenseKey("");
    setVerifiedEmail("");
  };

  const closeTrialModal = () => {
    setTrialModalOpen(false);
    setTrialStep("email");
    setEmail("");
    setOtp("");
    setError("");
  };

  const openPayModal = (plan: typeof PLANS[0]) => {
    setSelectedPlan(plan);
    setPayModalOpen(true);
    setPayStep(isLoggedIn ? "loading" : "email");
    setPayEmail(customerEmail || "");
    setError("");
  };

  const closePayModal = () => {
    setPayModalOpen(false);
    setSelectedPlan(null);
    setPayStep("email");
    setPayEmail("");
    setError("");
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
      await sendTrialOtp(em);
      setTrialStep("otp");
      setOtp("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to send code");
    } finally {
      setLoading(false);
    }
  };

  const handleVerifyOtp = async () => {
    const em = email.trim();
    if (!em || !em.includes("@")) {
      setError("Please enter a valid email");
      return;
    }
    if (!otp.trim() || otp.length !== 6) {
      setError("Please enter the 6-digit code from your email");
      return;
    }
    setError("");
    setLoading(true);
    try {
      const data = await verifyTrialOtp(em, otp.trim());
      if (data.success && data.token && data.license_key && data.email) {
        setLicenseKey(data.license_key);
        setVerifiedEmail(data.email);
        localStorage.setItem("customer_token", data.token);
        localStorage.setItem("customer_email", data.email);
        localStorage.setItem("customer_license", data.license_key);
        if (data.user_id) localStorage.setItem("customer_user_id", data.user_id);
        if (data.plan) localStorage.setItem("customer_plan", data.plan);
        setTrialStep("success");
      } else {
        setError(data.message || "Verification failed");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Verification failed");
    } finally {
      setLoading(false);
    }
  };

  const handleStartRazorpay = async () => {
    if (!selectedPlan) return;
    const em = (isLoggedIn ? customerEmail : payEmail)?.trim();
    if (!em || !em.includes("@")) {
      setError("Please enter a valid email");
      return;
    }
    setError("");
    setLoading(true);
    try {
      const res = await createSubscription(
        selectedPlan.id,
        em,
        customerUserId || undefined,
        customerLicense || undefined
      );
      const callbackUrl = `${window.location.origin}/pay/success`;
      const rzp = new window.Razorpay({
        key: res.key_id,
        subscription_id: res.subscription_id,
        name: "Ghost",
        description: `${selectedPlan.name} – ${selectedPlan.price}/mo`,
        callback_url: callbackUrl,
        prefill: { email: em },
        theme: { color: "#ff9a8b" },
      });
      rzp.open();
      closePayModal();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start checkout");
    } finally {
      setLoading(false);
    }
  };

  const handlePaySubmit = () => {
    if (isLoggedIn) {
      handleStartRazorpay();
    } else {
      const em = payEmail.trim();
      if (!em || !em.includes("@")) {
        setError("Please enter a valid email");
        return;
      }
      setError("");
      handleStartRazorpay();
    }
  };

  return (
    <div className="px-4 py-12 sm:px-6 sm:py-16 md:px-8 md:py-20 lg:px-12 lg:py-24 xl:px-32 xl:py-24" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
      <div className="max-w-4xl">
        <h1 className="mb-2 text-2xl sm:text-3xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>
          Subscription plans
        </h1>
        <p className="mb-6 sm:mb-8 text-white text-sm sm:text-base">
          Choose a plan for AI-powered chat, code, and analysis. Billed monthly.
        </p>

        {/* Free Trial section */}
        <div className="mb-8 rounded-lg border border-[#ff9a8b]/40 bg-[#ff9a8b]/10 p-4 sm:p-6">
          <h2 className="mb-2 text-lg font-semibold" style={{ color: "#ff9a8b" }}>14-day free trial</h2>
          <p className="mb-4 text-sm text-white/80">
            No credit card required. Verify your email to get started.
          </p>
          <button
            onClick={openTrialModal}
            disabled={loading}
            className="rounded-lg border border-[#ff9a8b] bg-[#ff9a8b]/20 px-6 py-2.5 font-medium text-white transition-colors hover:bg-[#ff9a8b]/30 disabled:cursor-not-allowed disabled:opacity-70"
          >
            Start Free Trial
          </button>
        </div>

        <p className="mb-6 text-sm text-white/80">
          Or subscribe to a paid plan:
        </p>
        {error && !trialModalOpen && !payModalOpen && (
          <div className="mb-4 text-sm text-red-400">{error}</div>
        )}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-6">
          {PLANS.map((plan) => (
            <div
              key={plan.id}
              className="min-w-0 rounded-lg border border-white/20 bg-white/5 p-4 sm:p-6"
            >
              <h3 className="mb-2 text-base sm:text-lg font-semibold" style={{ color: "#c96a5b" }}>{plan.name}</h3>
              <p className="mb-4 text-sm text-white/80">{plan.desc}</p>
              <p className="mb-6 text-2xl font-semibold text-white">{plan.price}/mo</p>
              <button
                onClick={() => openPayModal(plan)}
                disabled={loading}
                className="w-full rounded-lg border border-white bg-black px-4 py-3 font-medium text-white transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-70"
              >
                Subscribe
              </button>
            </div>
          ))}
        </div>
        <p className="mt-8 text-sm text-white/80">
          Already have an account?{" "}
          <Link to="/login" className="text-[#ff9a8b] hover:underline">Sign in</Link>
        </p>
      </div>

      {/* Free Trial OTP Modal */}
      {trialModalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
          onClick={(e) => e.target === e.currentTarget && closeTrialModal()}
        >
          <div
            className="w-full max-w-md rounded-xl border border-white/20 bg-black p-6 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-xl font-semibold" style={{ color: "#ff9a8b" }}>
                {trialStep === "email" && "Start your free trial"}
                {trialStep === "otp" && "Enter verification code"}
                {trialStep === "success" && "You're all set!"}
              </h2>
              <button
                onClick={closeTrialModal}
                className="text-white/60 hover:text-white text-2xl leading-none"
                aria-label="Close"
              >
                ×
              </button>
            </div>

            {trialStep === "email" && (
              <>
                <p className="mb-4 text-sm text-white/80">
                  Enter your email. We'll send a verification code from support@exora.solutions.
                </p>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="your@email.com"
                  className="mb-4 w-full rounded-lg border border-white/30 bg-black px-3 py-2.5 text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
                  autoFocus
                />
                {error && <p className="mb-4 text-sm text-red-400">{error}</p>}
                <div className="flex gap-3">
                  <button
                    onClick={handleSendOtp}
                    disabled={loading}
                    className="flex-1 rounded-lg border border-white bg-white px-4 py-2.5 font-medium text-black transition-colors hover:bg-white/90 disabled:opacity-70"
                  >
                    {loading ? "Sending..." : "Send code"}
                  </button>
                  <button
                    onClick={closeTrialModal}
                    className="rounded-lg border border-white/30 px-4 py-2.5 font-medium text-white hover:bg-white/10"
                  >
                    Cancel
                  </button>
                </div>
              </>
            )}

            {trialStep === "otp" && (
              <>
                <p className="mb-4 text-sm text-white/80">
                  We sent a 6-digit code to <span className="text-white">{email}</span>. Enter it below.
                </p>
                <input
                  type="text"
                  inputMode="numeric"
                  pattern="[0-9]*"
                  maxLength={6}
                  value={otp}
                  onChange={(e) => setOtp(e.target.value.replace(/\D/g, ""))}
                  placeholder="000000"
                  className="mb-4 w-full rounded-lg border border-white/30 bg-black px-3 py-2.5 text-center text-lg tracking-[0.5em] text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
                  autoFocus
                />
                {error && <p className="mb-4 text-sm text-red-400">{error}</p>}
                <div className="flex gap-3">
                  <button
                    onClick={handleVerifyOtp}
                    disabled={loading}
                    className="flex-1 rounded-lg border border-white bg-white px-4 py-2.5 font-medium text-black transition-colors hover:bg-white/90 disabled:opacity-70"
                  >
                    {loading ? "Verifying..." : "Verify"}
                  </button>
                  <button
                    onClick={() => setTrialStep("email")}
                    disabled={loading}
                    className="rounded-lg border border-white/30 px-4 py-2.5 font-medium text-white hover:bg-white/10"
                  >
                    Back
                  </button>
                </div>
                <button
                  onClick={handleSendOtp}
                  disabled={loading}
                  className="mt-3 w-full text-sm text-[#ff9a8b] hover:underline"
                >
                  Resend code
                </button>
              </>
            )}

            {trialStep === "success" && (
              <>
                <p className="mb-4 text-sm text-white/80">
                  Your 14-day free trial is active. Use your email <span className="text-white">{verifiedEmail}</span> in the Ghost app to sign in and get started.
                </p>
                <div className="mb-4 rounded-lg border border-white/20 bg-white/5 p-3">
                  <p className="mb-1 text-xs text-white/60">License key (save this)</p>
                  <p className="font-mono text-sm text-white break-all">{licenseKey}</p>
                </div>
                <button
                  onClick={() => { closeTrialModal(); navigate("/account", { replace: true }); }}
                  className="w-full rounded-lg border border-white bg-white px-4 py-2.5 font-medium text-black transition-colors hover:bg-white/90"
                >
                  Go to my account
                </button>
              </>
            )}
          </div>
        </div>
      )}

      {/* Razorpay / Pay Modal */}
      {payModalOpen && selectedPlan && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
          onClick={(e) => e.target === e.currentTarget && closePayModal()}
        >
          <div
            className="w-full max-w-md rounded-xl border border-white/20 bg-black p-6 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-xl font-semibold" style={{ color: "#ff9a8b" }}>
                Subscribe to {selectedPlan.name}
              </h2>
              <button
                onClick={closePayModal}
                className="text-white/60 hover:text-white text-2xl leading-none"
                aria-label="Close"
              >
                ×
              </button>
            </div>
            {payStep === "email" && (
              <>
                <p className="mb-4 text-sm text-white/80">
                  Enter your email to continue to Razorpay checkout.
                </p>
                <input
                  type="email"
                  value={payEmail}
                  onChange={(e) => setPayEmail(e.target.value)}
                  placeholder="your@email.com"
                  className="mb-4 w-full rounded-lg border border-white/30 bg-black px-3 py-2.5 text-white placeholder-white/50 focus:border-[#ff9a8b] focus:outline-none focus:ring-1 focus:ring-[#ff9a8b]"
                  autoFocus
                />
                {error && <p className="mb-4 text-sm text-red-400">{error}</p>}
                <div className="flex gap-3">
                  <button
                    onClick={handlePaySubmit}
                    disabled={loading}
                    className="flex-1 rounded-lg border border-white bg-white px-4 py-2.5 font-medium text-black transition-colors hover:bg-white/90 disabled:opacity-70"
                  >
                    {loading ? "Loading..." : "Continue to payment"}
                  </button>
                  <button
                    onClick={closePayModal}
                    className="rounded-lg border border-white/30 px-4 py-2.5 font-medium text-white hover:bg-white/10"
                  >
                    Cancel
                  </button>
                </div>
              </>
            )}
            {payStep === "loading" && isLoggedIn && (
              <>
                <p className="mb-4 text-sm text-white/80">
                  You'll be redirected to Razorpay to complete payment for {selectedPlan.name} ({selectedPlan.price}/mo).
                </p>
                <div className="flex gap-3">
                  <button
                    onClick={handlePaySubmit}
                    disabled={loading}
                    className="flex-1 rounded-lg border border-white bg-white px-4 py-2.5 font-medium text-black transition-colors hover:bg-white/90 disabled:opacity-70"
                  >
                    {loading ? "Loading..." : "Continue to payment"}
                  </button>
                  <button
                    onClick={closePayModal}
                    className="rounded-lg border border-white/30 px-4 py-2.5 font-medium text-white hover:bg-white/10"
                  >
                    Cancel
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
