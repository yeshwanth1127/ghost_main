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
        if (res.plan) localStorage.setItem("customer_plan", res.plan);
        if (res.license_key && localStorage.getItem("customer_token")) {
          localStorage.setItem("customer_license", res.license_key);
        }
      })
      .catch((err) => {
        setStatus("error");
        setMessage(err instanceof Error ? err.message : "Verification failed");
      });
  }, [searchParams]);

  if (status === "loading") {
    return (
      <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32">
        <div className="w-full max-w-md rounded-lg border border-white/20 bg-white/5 p-6 sm:p-8 mx-4" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
          <h1 className="mb-4 text-xl sm:text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>Verifying payment...</h1>
          <p className="text-white">Please wait.</p>
        </div>
      </div>
    );
  }

  if (status === "error") {
    return (
      <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32">
        <div className="w-full max-w-md rounded-lg border border-white/20 bg-white/5 p-6 sm:p-8 mx-4" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
          <h1 className="mb-4 text-xl sm:text-2xl font-bold text-red-400">Verification failed</h1>
          <p className="mb-6 text-white">{message}</p>
          <Link
            to="/subscriptions"
            className="inline-block rounded-lg border border-white bg-black px-6 py-3 font-medium text-white no-underline transition-colors hover:bg-white/10"
          >
            Back to subscriptions
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32">
      <div className="w-full max-w-md rounded-lg border border-white/20 bg-white/5 p-6 sm:p-8 mx-4" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
        <h1 className="mb-4 text-xl sm:text-2xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>Payment successful</h1>
        <p className="mb-4 text-white">{message}</p>
        {licenseKey && (
          <>
            <p className="mb-2 text-sm" style={{ color: "#c96a5b" }}>Your license key:</p>
            <code className="mb-4 block break-all rounded-lg bg-black p-4 text-sm text-white">
              {licenseKey}
            </code>
            <button
              onClick={() => navigator.clipboard.writeText(licenseKey)}
              className="mb-4 rounded-lg border border-white px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-white/10"
            >
              Copy
            </button>
          </>
        )}
        <div className="flex flex-wrap gap-3">
          <Link
            to="/"
            className="inline-block rounded-lg border border-white bg-black px-6 py-3 font-medium text-white no-underline transition-colors hover:bg-white/10"
          >
            Return to Ghost
          </Link>
          <Link
            to="/account"
            className="inline-block rounded-lg border border-white/30 bg-transparent px-6 py-3 font-medium text-white/80 no-underline transition-colors hover:text-white"
          >
            My Account
          </Link>
        </div>
      </div>
    </div>
  );
}
