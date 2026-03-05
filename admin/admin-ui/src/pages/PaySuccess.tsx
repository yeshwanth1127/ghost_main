import { useState, useEffect } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { verifyPayment } from "../api";

export default function PaySuccess() {
  const [searchParams] = useSearchParams();
  const [status, setStatus] = useState<"loading" | "success" | "error">("loading");
  const [licenseKey, setLicenseKey] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    const paymentId = searchParams.get("razorpay_payment_id");
    const subscriptionId = searchParams.get("razorpay_subscription_id");
    const signature = searchParams.get("razorpay_signature");

    if (!paymentId || !subscriptionId || !signature) {
      setStatus("error");
      setMessage("Missing payment details. Please try again from the subscriptions page.");
      return;
    }

    verifyPayment(paymentId, subscriptionId, signature)
      .then((res) => {
        setStatus("success");
        setLicenseKey(res.license_key || null);
        setMessage(res.message);
      })
      .catch((err) => {
        setStatus("error");
        setMessage(err instanceof Error ? err.message : "Verification failed");
      });
  }, [searchParams]);

  const containerStyle = {
    minHeight: "100vh",
    background: "#0f172a",
    color: "#e2e8f0",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    padding: "2rem",
  };

  const cardStyle = {
    padding: "2rem",
    background: "#1e293b",
    borderRadius: "8px",
    border: "1px solid #334155",
    maxWidth: "480px",
    width: "100%",
  };

  if (status === "loading") {
    return (
      <div style={containerStyle}>
        <div style={cardStyle}>
          <h1 style={{ margin: "0 0 1rem" }}>Verifying payment...</h1>
          <p style={{ color: "#94a3b8" }}>Please wait.</p>
        </div>
      </div>
    );
  }

  if (status === "error") {
    return (
      <div style={containerStyle}>
        <div style={cardStyle}>
          <h1 style={{ margin: "0 0 1rem", color: "#f87171" }}>Verification failed</h1>
          <p style={{ color: "#94a3b8", marginBottom: "1.5rem" }}>{message}</p>
          <Link
            to="/subscriptions"
            style={{
              display: "inline-block",
              padding: "0.75rem 1.5rem",
              background: "#3b82f6",
              borderRadius: "4px",
              color: "white",
              textDecoration: "none",
            }}
          >
            Back to subscriptions
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div style={containerStyle}>
      <div style={cardStyle}>
        <h1 style={{ margin: "0 0 1rem", color: "#22c55e" }}>Payment successful</h1>
        <p style={{ color: "#94a3b8", marginBottom: "1rem" }}>{message}</p>
        {licenseKey && (
          <>
            <p style={{ fontSize: "0.875rem", marginBottom: "0.5rem" }}>Your license key:</p>
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
              {licenseKey}
            </code>
            <button
              onClick={() => navigator.clipboard.writeText(licenseKey)}
              style={{
                padding: "0.5rem 1rem",
                background: "#334155",
                border: "none",
                borderRadius: "4px",
                color: "white",
                cursor: "pointer",
                marginRight: "0.5rem",
              }}
            >
              Copy
            </button>
          </>
        )}
        <div style={{ marginTop: "1.5rem" }}>
          <Link
            to="/"
            style={{
              display: "inline-block",
              padding: "0.75rem 1.5rem",
              background: "#3b82f6",
              borderRadius: "4px",
              color: "white",
              textDecoration: "none",
              marginRight: "0.5rem",
            }}
          >
            Return to Ghost
          </Link>
          <Link
            to="/account"
            style={{
              display: "inline-block",
              padding: "0.75rem 1.5rem",
              background: "transparent",
              borderRadius: "4px",
              color: "#94a3b8",
              textDecoration: "none",
              border: "1px solid #334155",
            }}
          >
            My Account
          </Link>
        </div>
      </div>
    </div>
  );
}
