import { Link, useNavigate } from "react-router-dom";

export default function Account() {
  const navigate = useNavigate();
  const token = localStorage.getItem("customer_token");
  const email = localStorage.getItem("customer_email");
  const licenseKey = localStorage.getItem("customer_license");

  if (!token) {
    navigate("/login", { replace: true });
    return null;
  }

  const handleLogout = () => {
    localStorage.removeItem("customer_token");
    localStorage.removeItem("customer_email");
    localStorage.removeItem("customer_license");
    localStorage.removeItem("customer_user_id");
    navigate("/", { replace: true });
  };

  const cardStyle = {
    padding: "1.5rem",
    background: "#1e293b",
    borderRadius: "8px",
    border: "1px solid #334155",
    marginBottom: "1rem",
  };

  return (
    <div style={{ minHeight: "100vh", background: "#0f172a", color: "#e2e8f0", padding: "2rem" }}>
      <div style={{ maxWidth: "600px", margin: "0 auto" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
          <h1 style={{ margin: 0 }}>My Account</h1>
          <button
            onClick={handleLogout}
            style={{
              padding: "0.5rem 1rem",
              background: "transparent",
              border: "1px solid #475569",
              borderRadius: "4px",
              color: "#94a3b8",
              cursor: "pointer",
            }}
          >
            Sign out
          </button>
        </div>
        <div style={cardStyle}>
          <h2 style={{ fontSize: "1rem", margin: "0 0 1rem" }}>Account details</h2>
          <p style={{ margin: "0 0 0.5rem", color: "#94a3b8" }}>Email</p>
          <p style={{ margin: "0 0 1rem" }}>{email || "—"}</p>
          <p style={{ margin: "0 0 0.5rem", color: "#94a3b8" }}>License key</p>
          <code
            style={{
              display: "block",
              padding: "0.75rem",
              background: "#0f172a",
              borderRadius: "4px",
              marginBottom: "1rem",
              wordBreak: "break-all",
              fontSize: "0.875rem",
            }}
          >
            {licenseKey || "—"}
          </code>
          <button
            onClick={() => licenseKey && navigator.clipboard.writeText(licenseKey)}
            style={{
              padding: "0.5rem 1rem",
              background: "#334155",
              border: "none",
              borderRadius: "4px",
              color: "white",
              cursor: "pointer",
            }}
          >
            Copy license
          </button>
        </div>
        <div style={cardStyle}>
          <h2 style={{ fontSize: "1rem", margin: "0 0 1rem" }}>Upgrade</h2>
          <p style={{ color: "#94a3b8", marginBottom: "1rem" }}>
            Get more tokens and access to premium models with a subscription.
          </p>
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
            View plans
          </Link>
        </div>
      </div>
    </div>
  );
}
