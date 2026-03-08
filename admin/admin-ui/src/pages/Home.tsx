import { Link } from "react-router-dom";

export default function Home() {
  return (
    <div className="bg-black">
      {/* Section 1: Hero */}
      <section
        id="hero"
        className="relative min-h-screen flex flex-col justify-center px-4 py-12 sm:px-6 sm:py-16 md:px-8 md:py-20 lg:px-12 lg:py-24 xl:px-32 xl:py-24"
      >
        {/* Branding and tagline */}
        <div className="flex flex-col justify-center max-w-4xl items-start">
          {/* Div 1: Logo + GHOST + by Exora */}
          <div className="inline-flex items-end gap-0.5 sm:gap-1 mb-6 sm:mb-8 -ml-5 sm:-ml-7 md:-ml-10 lg:-ml-12">
            <div
              className="w-20 h-20 sm:w-24 sm:h-24 md:w-32 md:h-32 lg:w-36 lg:h-36 xl:w-48 xl:h-48 flex-shrink-0 bg-[#ff9a8b] [mask-size:contain] [mask-repeat:no-repeat] [-webkit-mask-size:contain] [-webkit-mask-repeat:no-repeat]"
              style={{
                maskImage: "url(/ghost_logo.png)",
                WebkitMaskImage: "url(/ghost_logo.png)",
                maskPosition: "left center",
                WebkitMaskPosition: "left center",
              }}
              role="img"
              aria-label="Ghost"
            />
            <div className="flex flex-col justify-end pb-1">
              <h1
                className="text-5xl sm:text-6xl md:text-7xl lg:text-8xl xl:text-9xl font-bold tracking-tight leading-none"
                style={{
                  fontFamily: "Bebas Neue, sans-serif",
                  color: "transparent",
                  WebkitTextStroke: "1px #ff9a8b",
                }}
              >
                GHOST
              </h1>
              <p className="text-[10px] sm:text-xs mt-1 sm:mt-2" style={{ fontFamily: '"Press Start 2P", monospace', color: "#c96a5b" }}>
                by Exora
              </p>
            </div>
          </div>

          {/* Div 2: Tagline text */}
          <div>
            <p
              className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl xl:text-6xl font-medium leading-tight text-white max-w-2xl mt-2 sm:mt-3"
              style={{ fontFamily: "Space Grotesk, sans-serif" }}
            >
              Ghost turns your computer into an AI assistant.
            </p>
          </div>
        </div>

        {/* Scroll hint */}
        <a
          href="#use-cases"
          className="absolute bottom-6 sm:bottom-8 left-1/2 -translate-x-1/2 text-white/50 hover:text-white/80 transition-colors flex flex-col items-center gap-2"
          style={{ fontFamily: "Space Grotesk, sans-serif" }}
        >
          <span className="text-xs">Scroll to learn more</span>
          <svg className="w-5 h-5 animate-bounce" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
          </svg>
        </a>
      </section>

      {/* Section 2: Use Cases */}
      <section
        id="use-cases"
        className="min-h-screen flex flex-col justify-center px-4 py-20 sm:px-6 sm:py-24 md:px-8 md:py-28 lg:px-12 lg:py-32 xl:px-32 xl:py-40"
      >
        <h2
          className="text-4xl sm:text-5xl lg:text-6xl xl:text-7xl font-bold mb-12 sm:mb-16 md:mb-20"
          style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}
        >
          Use Cases
        </h2>
        <div
          className="grid gap-10 sm:gap-12 md:gap-16 lg:grid-cols-2 xl:grid-cols-4 lg:gap-16 xl:gap-20 max-w-6xl mb-12 sm:mb-16 md:mb-20"
          style={{ fontFamily: "Space Grotesk, sans-serif" }}
        >
          <div className="space-y-4 sm:space-y-5">
            <h3 className="text-xl sm:text-2xl font-semibold" style={{ color: "#c96a5b" }}>Students</h3>
            <ul className="space-y-2 text-white">
              <li>Homework help</li>
              <li>Lecture summaries</li>
              <li>Exam prep</li>
            </ul>
          </div>
          <div className="space-y-4 sm:space-y-5">
            <h3 className="text-xl sm:text-2xl font-semibold" style={{ color: "#c96a5b" }}>Developers</h3>
            <ul className="space-y-2 sm:space-y-3 text-white text-base sm:text-lg">
              <li>Debug code</li>
              <li>Explain errors</li>
              <li>Write functions instantly</li>
            </ul>
          </div>
          <div className="space-y-4 sm:space-y-5">
            <h3 className="text-xl sm:text-2xl font-semibold" style={{ color: "#c96a5b" }}>Creators</h3>
            <ul className="space-y-2 sm:space-y-3 text-white text-base sm:text-lg">
              <li>Summarize videos</li>
              <li>Brainstorm ideas</li>
              <li>Write scripts</li>
            </ul>
          </div>
          <div className="space-y-4 sm:space-y-5">
            <h3 className="text-xl sm:text-2xl font-semibold" style={{ color: "#c96a5b" }}>General</h3>
            <ul className="space-y-2 sm:space-y-3 text-white text-base sm:text-lg">
              <li>Summarize meetings in real time</li>
              <li>Answer questions on your screen</li>
              <li>Listen to your meetings and store it for later</li>
            </ul>
          </div>
        </div>
        <p className="text-lg sm:text-xl md:text-2xl lg:text-3xl text-white/90 max-w-3xl mb-16 sm:mb-20 md:mb-24" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
          It&apos;s always context aware of your computer, so you can talk to it like a real person.
        </p>
        <div className="mt-4">
          <Link
            to="/download"
            className="inline-block py-5 px-10 sm:py-6 sm:px-12 lg:py-7 lg:px-14 rounded-lg border border-white bg-black text-white text-center font-medium transition-colors hover:bg-white/5 text-base sm:text-lg"
            style={{ fontFamily: "Space Grotesk, sans-serif" }}
          >
            Get Started
          </Link>
        </div>
      </section>

      {/* Section 3: CTA */}
      <section
        id="cta"
        className="min-h-screen flex flex-col justify-center px-4 py-16 sm:px-6 sm:py-20 md:px-8 md:py-24 lg:px-12 lg:py-28 xl:px-32 xl:py-32"
      >
        <h2
          className="text-3xl sm:text-4xl md:text-5xl lg:text-6xl font-bold mb-4 sm:mb-6 max-w-4xl"
          style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}
        >
          Your computer is about to get smarter.
        </h2>
        <p
          className="text-lg sm:text-xl md:text-2xl lg:text-3xl text-white/90 max-w-3xl mb-10 sm:mb-16"
          style={{ fontFamily: "Space Grotesk, sans-serif" }}
        >
          Install Ghost and experience AI that actually understands what you&apos;re doing.
        </p>
        <Link
          to="/download"
          className="inline-block py-5 px-12 sm:py-6 sm:px-14 lg:py-7 lg:px-16 rounded-lg border border-white bg-black text-white text-center font-medium transition-colors hover:bg-white/5 text-lg sm:text-xl"
          style={{ fontFamily: "Space Grotesk, sans-serif" }}
        >
          Start Free
        </Link>
      </section>
    </div>
  );
}
