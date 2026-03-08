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

  if (status === "loading") {
    return (
      <div className="flex min-h-[calc(100vh-4rem)] items-center justify-center px-4 py-12">
        <div className="w-full max-w-md rounded-lg border border-ghost-border bg-ghost-surface p-8">
          <h1 className="mb-4 text-2xl font-semibold text-ghost-text">Verifying payment...</h1>
          <p className="text-ghost-muted">Please wait.</p>
        </div>
      </div>
    );
  }

  if (status === "error") {
    return (
      <div className="flex min-h-[calc(100vh-4rem)] items-center justify-center px-4 py-12">
        <div className="w-full max-w-md rounded-lg border border-ghost-border bg-ghost-surface p-8">
          <h1 className="mb-4 text-2xl font-semibold text-ghost-error">Verification failed</h1>
          <p className="mb-6 text-ghost-muted">{message}</p>
          <Link
            to="/subscriptions"
            className="inline-block rounded-md bg-ghost-accent px-6 py-3 font-medium text-white no-underline transition-colors hover:bg-blue-600"
          >
            Back to subscriptions
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-4rem)] items-center justify-center px-4 py-12">
      <div className="w-full max-w-md rounded-lg border border-ghost-border bg-ghost-surface p-8">
        <h1 className="mb-4 text-2xl font-semibold text-green-500">Payment successful</h1>
        <p className="mb-4 text-ghost-muted">{message}</p>
        {licenseKey && (
          <>
            <p className="mb-2 text-sm text-ghost-muted">Your license key:</p>
            <code className="mb-4 block break-all rounded-md bg-ghost-bg p-4 text-sm text-ghost-text">
              {licenseKey}
            </code>
            <button
              onClick={() => navigator.clipboard.writeText(licenseKey)}
              className="mb-4 rounded-md bg-ghost-border px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-ghost-muted"
            >
              Copy
            </button>
          </>
        )}
        <div className="flex flex-wrap gap-3">
          <Link
            to="/"
            className="inline-block rounded-md bg-ghost-accent px-6 py-3 font-medium text-white no-underline transition-colors hover:bg-blue-600"
          >
            Return to Ghost
          </Link>
          <Link
            to="/account"
            className="inline-block rounded-md border border-ghost-border bg-transparent px-6 py-3 font-medium text-ghost-muted no-underline transition-colors hover:text-ghost-text"
          >
            My Account
          </Link>
        </div>
      </div>
    </div>
  );
}
