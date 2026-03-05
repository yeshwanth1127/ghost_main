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

const cardStyle = {
  padding: "1.5rem",
  background: "#1e293b",
  borderRadius: "8px",
  border: "1px solid #334155",
  flex: "1",
  minWidth: "180px",
};

export default function Subscriptions() {
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [scriptLoaded, setScriptLoaded] = useState(false);

  const customerEmail = localStorage.getItem("customer_email");
  const customerToken = localStorage.getItem("customer_token");
  const customerLicense = localStorage.getItem("customer_license");
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
        customerLicense || undefined
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
    <div style={{ minHeight: "100vh", background: "#0f172a", color: "#e2e8f0", padding: "2rem" }}>
      <div style={{ maxWidth: "900px", margin: "0 auto" }}>
        <h1 style={{ marginBottom: "0.5rem" }}>Subscription plans</h1>
        <p style={{ color: "#94a3b8", marginBottom: "2rem" }}>
          Choose a plan to unlock more tokens and premium models. Billed monthly.
        </p>
        {!customerToken && (
          <p style={{ marginBottom: "1rem", fontSize: "0.875rem" }}>
            <Link to="/signup" style={{ color: "#3b82f6" }}>Sign up</Link> for a 14-day free trial, or{" "}
            <Link to="/login" style={{ color: "#3b82f6" }}>sign in</Link> if you have an account.
          </p>
        )}
        <div style={{ marginBottom: "1.5rem" }}>
          <label style={{ display: "block", marginBottom: "0.5rem", fontSize: "0.875rem" }}>Email</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="your@email.com"
            style={{
              width: "100%",
              maxWidth: "320px",
              padding: "0.5rem 0.75rem",
              background: "#0f172a",
              border: "1px solid #334155",
              borderRadius: "4px",
              color: "#e2e8f0",
              fontSize: "1rem",
            }}
          />
        </div>
        {error && (
          <div style={{ color: "#f87171", marginBottom: "1rem", fontSize: "0.875rem" }}>{error}</div>
        )}
        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          {PLANS.map((plan) => (
            <div key={plan.id} style={cardStyle}>
              <h3 style={{ margin: "0 0 0.5rem" }}>{plan.name}</h3>
              <p style={{ color: "#94a3b8", fontSize: "0.875rem", margin: "0 0 1rem" }}>{plan.desc}</p>
              <p style={{ fontSize: "1.5rem", fontWeight: 600, margin: "0 0 0.5rem" }}>{plan.price}/mo</p>
              <p style={{ fontSize: "0.875rem", color: "#94a3b8", margin: "0 0 1rem" }}>{plan.tokens} tokens</p>
              <button
                onClick={() => handleSubscribe(plan.id)}
                disabled={!!loading}
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
                {loading === plan.id ? "Opening..." : "Subscribe"}
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
