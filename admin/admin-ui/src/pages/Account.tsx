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

  return (
    <div className="px-4 py-12 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-2xl">
        <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <h1 className="text-2xl font-bold text-ghost-text">My Account</h1>
          <button
            onClick={handleLogout}
            className="rounded-md border border-ghost-border bg-transparent px-4 py-2 text-sm font-medium text-ghost-muted transition-colors hover:bg-ghost-surface hover:text-ghost-text"
          >
            Sign out
          </button>
        </div>
        <div className="mb-6 rounded-lg border border-ghost-border bg-ghost-surface p-6">
          <h2 className="mb-4 text-base font-semibold text-ghost-text">Account details</h2>
          <p className="mb-1 text-sm text-ghost-muted">Email</p>
          <p className="mb-4">{email || "—"}</p>
          <p className="mb-1 text-sm text-ghost-muted">License key</p>
          <code className="mb-4 block break-all rounded-md bg-ghost-bg px-3 py-2 text-sm text-ghost-text">
            {licenseKey || "—"}
          </code>
          <button
            onClick={() => licenseKey && navigator.clipboard.writeText(licenseKey)}
            className="rounded-md bg-ghost-border px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-ghost-muted"
          >
            Copy license
          </button>
        </div>
        <div className="rounded-lg border border-ghost-border bg-ghost-surface p-6">
          <h2 className="mb-4 text-base font-semibold text-ghost-text">Upgrade</h2>
          <p className="mb-6 text-ghost-muted">
            Get more tokens and access to premium models with a subscription.
          </p>
          <Link
            to="/subscriptions"
            className="inline-block rounded-md bg-ghost-accent px-6 py-3 font-medium text-white no-underline transition-colors hover:bg-blue-600"
          >
            View plans
          </Link>
        </div>
      </div>
    </div>
  );
}
