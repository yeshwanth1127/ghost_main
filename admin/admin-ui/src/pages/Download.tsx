import { Link } from "react-router-dom";

const VERSION = "0.1.7";
const DOWNLOAD_BASE = "/desktop";

const downloads = [
  {
    platform: "Windows",
    icon: "🪟",
    files: [
      { label: "Installer (.exe)", path: `Ghost_${VERSION}_x64-setup.exe` },
      { label: "MSI", path: `Ghost_${VERSION}_x64_en-US.msi` },
    ],
  },
  {
    platform: "macOS",
    icon: "🍎",
    files: [
      { label: "Apple Silicon (M1/M2/M3)", path: `Ghost_${VERSION}_aarch64.dmg` },
      { label: "Intel", path: `Ghost_${VERSION}_x86_64.dmg` },
    ],
  },
  {
    platform: "Linux",
    icon: "🐧",
    files: [
      { label: "Debian/Ubuntu (.deb)", path: `ghost_${VERSION}_amd64.deb` },
      { label: "AppImage", path: `Ghost_${VERSION}_amd64.AppImage` },
    ],
  },
];

export default function Download() {
  return (
    <div className="px-4 py-16 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-3xl">
        <h1 className="mb-4 text-4xl font-bold text-ghost-text">Download Ghost</h1>
        <p className="mb-12 text-lg text-ghost-muted">
          Get the Ghost desktop app for your platform. Install and connect to start using AI-powered chat, code, and analysis.
        </p>

        <div className="space-y-8">
          {downloads.map(({ platform, icon, files }) => (
            <div
              key={platform}
              className="rounded-xl border border-ghost-border bg-ghost-surface p-6"
            >
              <h2 className="mb-4 flex items-center gap-2 text-xl font-semibold text-ghost-text">
                <span>{icon}</span>
                {platform}
              </h2>
              <div className="flex flex-wrap gap-3">
                {files.map(({ label, path }) => (
                  <a
                    key={path}
                    href={`${DOWNLOAD_BASE}/${path}`}
                    download
                    className="inline-flex items-center rounded-lg bg-ghost-accent px-5 py-3 font-medium text-white transition-colors hover:bg-blue-600"
                  >
                    {label}
                  </a>
                ))}
              </div>
            </div>
          ))}
        </div>

        <p className="mt-12 text-sm text-ghost-muted">
          After installing, open Ghost and sign in or start a free trial. Need a subscription?{" "}
          <Link to="/subscriptions" className="text-ghost-accent hover:underline">
            View plans
          </Link>
        </p>
      </div>
    </div>
  );
}
