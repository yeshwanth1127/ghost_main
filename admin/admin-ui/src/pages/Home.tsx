import { Link } from "react-router-dom";
import PixelBlast from "../components/PixelBlast";

const features = [
  {
    title: "Powerful models",
    description: "Access GPT-4, Claude, Gemini, and more. Choose the right model for every task.",
  },
  {
    title: "Scribe desktop app",
    description: "Chat, code, and analyze with a native desktop experience. Fast and responsive.",
  },
  {
    title: "Flexible subscriptions",
    description: "Start with a 14-day free trial. Scale tokens and premium models as you need.",
  },
  {
    title: "Privacy-focused",
    description: "Your data stays yours. Use Ghost with confidence for sensitive workflows.",
  },
];

export default function Home() {
  const isAdmin = !!localStorage.getItem("admin_token");
  const isCustomer = !!localStorage.getItem("customer_token");

  return (
    <div>
      {/* Hero */}
      <section className="relative min-h-[600px] px-4 py-20 sm:py-28 sm:px-6 lg:px-8">
        <div className="absolute inset-0 min-h-[600px] w-full overflow-hidden">
          <PixelBlast
            variant="square"
            pixelSize={4}
            color="#B19EEF"
            patternScale={2}
            patternDensity={1}
            pixelSizeJitter={0}
            enableRipples
            rippleSpeed={0.4}
            rippleThickness={0.12}
            rippleIntensityScale={1.5}
            liquid={false}
            liquidStrength={0.12}
            liquidRadius={1.2}
            liquidWobbleSpeed={5}
            speed={0.5}
            edgeFade={0.25}
            transparent
          />
        </div>
        <div className="relative z-10 mx-auto max-w-3xl text-center">
          <h1 className="text-4xl font-bold tracking-tight text-ghost-text sm:text-5xl lg:text-6xl">
            Ghost
          </h1>
          <p className="mt-6 text-lg leading-8 text-ghost-muted sm:text-xl">
            AI-powered desktop assistant that brings powerful language models to your workflow.
          </p>
          <p className="mt-4 text-base leading-7 text-ghost-muted">
            Use Scribe for chat, code, and analysis with models like GPT-4, Claude, and Gemini.
          </p>
          <div className="mt-10 flex flex-wrap justify-center gap-4">
            {!isAdmin && !isCustomer && (
              <>
                <Link
                  to="/signup"
                  className="rounded-lg bg-ghost-accent px-6 py-3 text-base font-semibold text-white shadow-sm hover:bg-blue-600 transition-colors"
                >
                  Get started
                </Link>
                <Link
                  to="/subscriptions"
                  className="rounded-lg border border-ghost-border bg-transparent px-6 py-3 text-base font-semibold text-ghost-muted hover:text-ghost-text hover:border-ghost-muted transition-colors"
                >
                  View plans
                </Link>
                <Link
                  to="/login"
                  className="rounded-lg border border-ghost-border bg-transparent px-6 py-3 text-base font-semibold text-ghost-muted hover:text-ghost-text hover:border-ghost-muted transition-colors"
                >
                  Sign in
                </Link>
              </>
            )}
            {isAdmin && (
              <Link
                to="/dashboard"
                className="rounded-lg bg-ghost-accent px-6 py-3 text-base font-semibold text-white shadow-sm hover:bg-blue-600 transition-colors"
              >
                Admin Dashboard
              </Link>
            )}
            {isCustomer && (
              <Link
                to="/account"
                className="rounded-lg bg-ghost-accent px-6 py-3 text-base font-semibold text-white shadow-sm hover:bg-blue-600 transition-colors"
              >
                My Account
              </Link>
            )}
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="border-t border-ghost-border px-4 py-16 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-5xl">
          <h2 className="text-center text-2xl font-semibold text-ghost-text sm:text-3xl">
            Built for productivity
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-center text-ghost-muted">
            Everything you need to work smarter with AI.
          </p>
          <div className="mt-12 grid gap-8 sm:grid-cols-2 lg:grid-cols-4">
            {features.map((feature) => (
              <div
                key={feature.title}
                className="rounded-xl border border-ghost-border bg-ghost-surface p-6"
              >
                <h3 className="text-lg font-semibold text-ghost-text">{feature.title}</h3>
                <p className="mt-2 text-sm leading-6 text-ghost-muted">{feature.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="border-t border-ghost-border px-4 py-16 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-2xl font-semibold text-ghost-text sm:text-3xl">
            Ready to get started?
          </h2>
          <p className="mt-4 text-ghost-muted">
            Subscribe to unlock more tokens and premium models. 14-day free trial.
          </p>
          <div className="mt-8">
            <Link
              to="/subscriptions"
              className="rounded-lg bg-ghost-accent px-6 py-3 text-base font-semibold text-white shadow-sm hover:bg-blue-600 transition-colors"
            >
              View subscription plans
            </Link>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-ghost-border px-4 py-8 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-6xl flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-sm text-ghost-muted">Ghost – AI Desktop Assistant</p>
          <div className="flex gap-6">
            <Link to="/subscriptions" className="text-sm text-ghost-muted hover:text-ghost-text transition-colors">
              Plans
            </Link>
            <Link to="/login" className="text-sm text-ghost-muted hover:text-ghost-text transition-colors">
              Sign in
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
