import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { createSubscription } from "../api";

declare global {
  interface Window {
    Razorpay: new (options: Record<string, unknown>) => { open: () => void };
  }
}

const PLANS = [
  { id: "starter", name: "Starter", tokens: "500K", price: "₹499", desc: "For light usage" },
  { id: "pro", name: "Pro", tokens: "1M", price: "₹999", desc: "For power users" },
  { id: "power", name: "Power", tokens: "2M", price: "₹1,999", desc: "Maximum capacity" },
];

export default function Subscriptions() {
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [scriptLoaded, setScriptLoaded] = useState(false);

  const customerEmail = localStorage.getItem("customer_email");
  const customerToken = localStorage.getItem("customer_token");
  const customerUserId = localStorage.getItem("customer_user_id");

  useEffect(() => {
    if (customerEmail) setEmail(customerEmail);
  }, [customerEmail]);

  useEffect(() => {
    const script = document.createElement("script");
    script.src = "https://checkout.razorpay.com/v1/checkout.js";
    script.async = true;
    script.onload = () => setScriptLoaded(true);
    document.body.appendChild(script);
    return () => {
      document.body.removeChild(script);
    };
  }, []);

  const handleSubscribe = async (planId: string) => {
    const em = email.trim();
    if (!em || !em.includes("@")) {
      setError("Please enter a valid email");
      return;
    }
    if (!scriptLoaded) {
      setError("Payment system loading...");
      return;
    }
    setError("");
    setLoading(planId);
    try {
      const { subscription_id, key_id } = await createSubscription(
        planId,
        em,
        customerUserId || undefined,
        localStorage.getItem("customer_license") || undefined
      );
      const callbackUrl = `${window.location.origin}/pay/success`;
      const options = {
        key: key_id,
        subscription_id,
        name: "Ghost",
        description: `${PLANS.find((p) => p.id === planId)?.name} Plan`,
        callback_url: callbackUrl,
        prefill: { email: em },
        notes: { email: em },
        theme: { color: "#3b82f6" },
      };
      const rzp = new window.Razorpay(options);
      rzp.open();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create subscription");
    } finally {
      setLoading(null);
    }
  };

  return (
    <div className="px-4 py-12 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-4xl">
        <h1 className="mb-2 text-3xl font-bold text-ghost-text">Subscription plans</h1>
        <p className="mb-8 text-ghost-muted">
          Choose a plan to unlock more tokens and premium models. Billed monthly.
        </p>
        {!customerToken && (
          <p className="mb-6 text-sm text-ghost-muted">
            <Link to="/signup" className="text-ghost-accent hover:underline">Sign up</Link>
            {" "}for a 14-day free trial, or{" "}
            <Link to="/login" className="text-ghost-accent hover:underline">sign in</Link>
            {" "}if you have an account.
          </p>
        )}
        <div className="mb-6">
          <label className="mb-2 block text-sm text-ghost-muted">Email</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="your@email.com"
            className="max-w-md w-full rounded-md border border-ghost-border bg-ghost-bg px-3 py-2 text-ghost-text placeholder-ghost-muted focus:border-ghost-accent focus:outline-none focus:ring-1 focus:ring-ghost-accent"
          />
        </div>
        {error && (
          <div className="mb-4 text-sm text-ghost-error">{error}</div>
        )}
        <div className="flex flex-wrap gap-6">
          {PLANS.map((plan) => (
            <div
              key={plan.id}
              className="min-w-[180px] flex-1 rounded-lg border border-ghost-border bg-ghost-surface p-6"
            >
              <h3 className="mb-2 text-lg font-semibold text-ghost-text">{plan.name}</h3>
              <p className="mb-4 text-sm text-ghost-muted">{plan.desc}</p>
              <p className="mb-1 text-2xl font-semibold text-ghost-text">{plan.price}/mo</p>
              <p className="mb-6 text-sm text-ghost-muted">{plan.tokens} tokens</p>
              <button
                onClick={() => handleSubscribe(plan.id)}
                disabled={!!loading}
                className="w-full rounded-md bg-ghost-accent px-4 py-3 font-medium text-white transition-colors hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-70"
              >
                {loading === plan.id ? "Opening..." : "Subscribe"}
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
