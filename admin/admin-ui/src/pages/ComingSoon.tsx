import { Link } from "react-router-dom";

export default function ComingSoon() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 lg:px-12 xl:px-32" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
      <h1
        className="mb-4 text-4xl sm:text-5xl md:text-6xl font-bold"
        style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}
      >
        Coming Soon
      </h1>
      <p className="mb-8 text-center text-lg sm:text-xl text-white/90 max-w-md">
        We&apos;re working on the demo. Check back soon to see Ghost in action.
      </p>
      <Link
        to="/"
        className="inline-block rounded-lg border border-white bg-black px-6 py-3 font-medium text-white transition-colors hover:bg-white/10"
      >
        Back to Home
      </Link>
    </div>
  );
}
