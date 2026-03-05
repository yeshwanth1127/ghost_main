import { Link } from "react-router-dom";

const styles = {
  page: {
    minHeight: "100vh",
    background: "#0f172a",
    color: "#e2e8f0",
    padding: "2rem",
  },
  container: {
    maxWidth: "800px",
    margin: "0 auto",
  },
  h1: { fontSize: "2rem", marginBottom: "1rem" },
  p: { marginBottom: "1rem", lineHeight: 1.6, color: "#94a3b8" },
  buttons: { display: "flex", gap: "1rem", marginTop: "2rem", flexWrap: "wrap" as const },
  btn: {
    padding: "0.75rem 1.5rem",
    borderRadius: "8px",
    textDecoration: "none",
    fontWeight: 600,
    display: "inline-block",
  },
  btnPrimary: {
    background: "#3b82f6",
    color: "white",
    border: "none",
  },
  btnSecondary: {
    background: "transparent",
    color: "#94a3b8",
    border: "1px solid #334155",
  },
};

export default function Home() {
  const isAdmin = !!localStorage.getItem("admin_token");
  const isCustomer = !!localStorage.getItem("customer_token");

  return (
    <div style={styles.page}>
      <div style={styles.container}>
        <h1 style={styles.h1}>Ghost</h1>
        <p style={styles.p}>
          Ghost is an AI-powered desktop assistant that brings powerful language models to your workflow.
          Use Scribe for chat, code, and analysis with models like GPT-4, Claude, and Gemini.
        </p>
        <p style={styles.p}>
          Subscribe to unlock more tokens and premium models. Start with a 14-day free trial.
        </p>

        <div style={styles.buttons}>
          {!isAdmin && !isCustomer && (
            <>
              <Link to="/login" style={{ ...styles.btn, ...styles.btnPrimary }}>
                Sign in
              </Link>
              <Link to="/signup" style={{ ...styles.btn, ...styles.btnSecondary }}>
                Sign up
              </Link>
            </>
          )}
          {isAdmin && (
            <Link to="/dashboard" style={{ ...styles.btn, ...styles.btnPrimary }}>
              Admin Dashboard
            </Link>
          )}
          {isCustomer && (
            <Link to="/account" style={{ ...styles.btn, ...styles.btnPrimary }}>
              My Account
            </Link>
          )}
          <Link to="/subscriptions" style={{ ...styles.btn, ...styles.btnSecondary }}>
            View subscription plans
          </Link>
        </div>
      </div>
    </div>
  );
}
