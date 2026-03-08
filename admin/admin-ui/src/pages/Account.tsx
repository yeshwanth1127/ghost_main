import { Link, useNavigate } from "react-router-dom";

export default function Account() {
  const navigate = useNavigate();
  const token = localStorage.getItem("customer_token");
  const email = localStorage.getItem("customer_email");
  const licenseKey = localStorage.getItem("customer_license");
  const plan = localStorage.getItem("customer_plan");

  const planLabel = plan ? ({ free: "Free Trial", starter: "Starter", pro: "Pro", power: "Power" } as Record<string, string>)[plan.toLowerCase()] ?? plan : "—";

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
    <div className="px-4 py-12 sm:px-6 sm:py-16 md:px-8 md:py-20 lg:px-12 lg:py-24 xl:px-32 xl:py-24" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
      <div className="max-w-2xl">
        <div className="mb-6 sm:mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <h1 className="text-xl sm:text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>
            My Account
          </h1>
          <button
            onClick={handleLogout}
            className="rounded-lg border border-white/30 bg-transparent px-4 py-2 text-sm font-medium text-white/80 transition-colors hover:bg-white/10 hover:text-white"
          >
            Sign out
          </button>
        </div>
        <div className="mb-6 rounded-lg border border-white/20 bg-white/5 p-4 sm:p-6">
          <h2 className="mb-4 text-base font-semibold" style={{ color: "#c96a5b" }}>Account details</h2>
          <p className="mb-1 text-sm text-white/80">Email</p>
          <p className="mb-4 text-white">{email || "—"}</p>
          <p className="mb-1 text-sm text-white/80">Plan</p>
          <p className="mb-4 text-white">{planLabel}</p>
          <p className="mb-1 text-sm text-white/80">License key</p>
          <code className="mb-4 block break-all rounded-lg bg-black px-3 py-2 text-sm text-white">
            {licenseKey || "—"}
          </code>
          <button
            onClick={() => licenseKey && navigator.clipboard.writeText(licenseKey)}
            className="rounded-lg border border-white px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-white/10"
          >
            Copy license
          </button>
        </div>
        <div className="rounded-lg border border-white/20 bg-white/5 p-4 sm:p-6">
          <h2 className="mb-4 text-base font-semibold" style={{ color: "#c96a5b" }}>Upgrade</h2>
          <p className="mb-6 text-white">
            Get more tokens and access to premium models with a subscription.
          </p>
          <Link
            to="/subscriptions"
            className="inline-block rounded-lg border border-white bg-black px-6 py-3 font-medium text-white no-underline transition-colors hover:bg-white/10"
          >
            View plans
          </Link>
        </div>
      </div>
    </div>
  );
}
