import { Link, useLocation } from "react-router-dom";

const navButtonClass =
  "py-2.5 px-5 rounded-lg border border-white bg-black text-white text-center font-medium transition-colors hover:bg-white/5 text-sm";
const navButtonStyle = { fontFamily: "Space Grotesk, sans-serif" };

export default function Navbar() {
  const location = useLocation();
  const isAdmin = !!localStorage.getItem("admin_token");
  const isCustomer = !!localStorage.getItem("customer_token");

  const isActive = (path: string) => {
    if (path === "/") return location.pathname === "/";
    return location.pathname.startsWith(path);
  };

  return (
    <nav className="sticky top-0 z-50 w-full border-b border-white/20 bg-black backdrop-blur-sm">
      <div className="flex h-16 items-center justify-between px-8 lg:px-16">
        {/* Left: Logo + GHOST */}
        <Link
          to="/"
          className="flex items-center gap-3"
          style={{ fontFamily: "Space Grotesk, sans-serif" }}
        >
          <img
            src="/ghost_logo.png"
            alt="Ghost"
            className="h-10 w-10 lg:h-12 lg:w-12 object-contain"
          />
          <div>
            <span
              className="text-2xl lg:text-3xl font-bold tracking-tight"
              style={{
                fontFamily: "Bebas Neue, sans-serif",
                color: "#ff9a8b",
              }}
            >
              GHOST
            </span>
            <p
              className="text-[10px] lg:text-xs"
              style={{ fontFamily: '"Press Start 2P", monospace', color: "#c96a5b" }}
            >
              by Exora
            </p>
          </div>
        </Link>

        {/* Right: Three buttons */}
        <div className="flex flex-wrap items-center justify-end gap-3">
          <Link
            to="/download"
            className={`${navButtonClass} ${isActive("/download") ? "bg-white/10" : ""}`}
            style={navButtonStyle}
          >
            Download
          </Link>
          <Link
            to="/subscriptions"
            className={`${navButtonClass} ${isActive("/subscriptions") ? "bg-white/10" : ""}`}
            style={navButtonStyle}
          >
            Subscriptions
          </Link>
          {isAdmin ? (
            <Link
              to="/dashboard"
              className={`${navButtonClass} ${isActive("/dashboard") ? "bg-white/10" : ""}`}
              style={navButtonStyle}
            >
              Admin
            </Link>
          ) : isCustomer ? (
            <Link
              to="/account"
              className={`${navButtonClass} ${isActive("/account") ? "bg-white/10" : ""}`}
              style={navButtonStyle}
            >
              Account
            </Link>
          ) : (
            <Link
              to="/login"
              className={`${navButtonClass} ${isActive("/login") ? "bg-white/10" : ""}`}
              style={navButtonStyle}
            >
              Login / Sign Up
            </Link>
          )}
        </div>
      </div>
    </nav>
  );
}
