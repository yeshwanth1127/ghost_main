import { Link } from "react-router-dom";
import { WindowsIcon, AppleIcon, LinuxIcon } from "../components/PlatformIcons";

const VERSION = "0.1.7";
const DOWNLOAD_BASE = "/desktop";

const downloads = [
  {
    platform: "Windows",
    Icon: WindowsIcon,
    files: [
      { label: "Installer (.exe)", path: `Ghost_${VERSION}_x64-setup.exe` },
      { label: "MSI", path: `Ghost_${VERSION}_x64_en-US.msi` },
    ],
  },
  {
    platform: "macOS",
    Icon: AppleIcon,
    files: [
      { label: "Apple Silicon (M1/M2/M3)", path: `Ghost_${VERSION}_aarch64.dmg` },
      { label: "Intel", path: `Ghost_${VERSION}_x86_64.dmg` },
    ],
  },
  {
    platform: "Linux",
    Icon: LinuxIcon,
    files: [
      { label: "Debian/Ubuntu (.deb)", path: `ghost_${VERSION}_amd64.deb` },
      { label: "AppImage", path: `Ghost_${VERSION}_amd64.AppImage` },
    ],
  },
];

export default function Download() {
  return (
    <div className="px-4 py-12 sm:px-6 sm:py-16 md:px-8 md:py-20 lg:px-12 lg:py-24 xl:px-32 xl:py-24" style={{ fontFamily: "Space Grotesk, sans-serif" }}>
      <div className="max-w-3xl">
        <h1 className="mb-4 text-3xl sm:text-4xl font-bold" style={{ fontFamily: "Bebas Neue, sans-serif", color: "#ff9a8b" }}>
          Download Ghost
        </h1>
        <p className="mb-8 sm:mb-12 text-base sm:text-lg text-white">
          Get the Ghost desktop app for your platform. Install and connect to start using AI-powered chat, code, and analysis.
        </p>

        <div className="space-y-6 sm:space-y-8">
          {downloads.map(({ platform, Icon, files }) => (
            <div
              key={platform}
              className="rounded-xl border border-white/20 bg-white/5 p-4 sm:p-6"
            >
              <h2 className="mb-3 sm:mb-4 flex items-center gap-2 text-lg sm:text-xl font-semibold" style={{ color: "#c96a5b" }}>
                <Icon className="w-6 h-6 sm:w-7 sm:h-7 flex-shrink-0" />
                {platform}
              </h2>
              <div className="flex flex-wrap gap-3">
                {files.map(({ label, path }) => (
                  <a
                    key={path}
                    href={`${DOWNLOAD_BASE}/${path}`}
                    download
                    className="inline-flex items-center rounded-lg border border-white bg-black px-4 py-2.5 sm:px-5 sm:py-3 font-medium text-white text-sm sm:text-base transition-colors hover:bg-white/10"
                  >
                    {label}
                  </a>
                ))}
              </div>
            </div>
          ))}
        </div>

        <p className="mt-8 sm:mt-12 text-sm text-white/80">
          After installing, open Ghost and sign in or start a free trial. Need a subscription?{" "}
          <Link to="/subscriptions" className="text-[#ff9a8b] hover:underline">
            View plans
          </Link>
        </p>
      </div>
    </div>
  );
}
