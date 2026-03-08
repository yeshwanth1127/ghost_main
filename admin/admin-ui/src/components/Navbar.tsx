import { useState } from "react";
import { Link, useLocation } from "react-router-dom";

const navLinks = [
  { to: "/", label: "Home" },
  { to: "/download", label: "Download" },
  { to: "/subscriptions", label: "Subscriptions" },
];

export default function Navbar() {
  const [mobileOpen, setMobileOpen] = useState(false);
  const location = useLocation();
  const isAdmin = !!localStorage.getItem("admin_token");
  const isCustomer = !!localStorage.getItem("customer_token");

  const isActive = (path: string) => {
    if (path === "/") return location.pathname === "/";
    return location.pathname.startsWith(path);
  };

  const NavLink = ({ to, label }: { to: string; label: string }) => (
    <Link
      to={to}
      onClick={() => setMobileOpen(false)}
      className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
        isActive(to)
          ? "bg-ghost-surface text-ghost-text"
          : "text-ghost-muted hover:text-ghost-text hover:bg-ghost-surface/50"
      }`}
    >
      {label}
    </Link>
  );

  return (
    <nav className="sticky top-0 z-50 w-full border-b border-ghost-border bg-ghost-bg/80 backdrop-blur-md">
      <div className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between">
          <Link
            to="/"
            className="flex items-center gap-2 text-xl font-semibold text-ghost-text hover:text-white transition-colors"
            onClick={() => setMobileOpen(false)}
          >
            Ghost
          </Link>

          {/* Desktop nav */}
          <div className="hidden md:flex md:items-center md:gap-1">
            {navLinks.map((link) => (
              <NavLink key={link.to} to={link.to} label={link.label} />
            ))}
            {isAdmin && <NavLink to="/dashboard" label="Dashboard" />}
            {isCustomer && <NavLink to="/account" label="Account" />}
            {!isAdmin && !isCustomer && (
              <>
                <Link
                  to="/login"
                  onClick={() => setMobileOpen(false)}
                  className="ml-2 px-4 py-2 text-sm font-medium text-ghost-muted hover:text-ghost-text transition-colors"
                >
                  Sign in
                </Link>
                <Link
                  to="/signup"
                  onClick={() => setMobileOpen(false)}
                  className="ml-2 px-4 py-2 rounded-md text-sm font-medium bg-ghost-accent text-white hover:bg-blue-600 transition-colors"
                >
                  Sign up
                </Link>
              </>
            )}
          </div>

          {/* Mobile menu button */}
          <button
            type="button"
            className="md:hidden inline-flex items-center justify-center p-2 rounded-md text-ghost-muted hover:text-ghost-text hover:bg-ghost-surface"
            onClick={() => setMobileOpen(!mobileOpen)}
            aria-expanded={mobileOpen}
          >
            <span className="sr-only">Open menu</span>
            <svg
              className="h-6 w-6"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              {mobileOpen ? (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              ) : (
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
              )}
            </svg>
          </button>
        </div>

        {/* Mobile menu */}
        {mobileOpen && (
          <div className="md:hidden py-4 space-y-1 border-t border-ghost-border">
            {navLinks.map((link) => (
              <NavLink key={link.to} to={link.to} label={link.label} />
            ))}
            {isAdmin && <NavLink to="/dashboard" label="Dashboard" />}
            {isCustomer && <NavLink to="/account" label="Account" />}
            {!isAdmin && !isCustomer && (
              <div className="pt-4 space-y-2">
                <Link
                  to="/login"
                  onClick={() => setMobileOpen(false)}
                  className="block px-3 py-2 rounded-md text-ghost-muted hover:text-ghost-text hover:bg-ghost-surface"
                >
                  Sign in
                </Link>
                <Link
                  to="/signup"
                  onClick={() => setMobileOpen(false)}
                  className="block px-3 py-2 rounded-md bg-ghost-accent text-white font-medium text-center"
                >
                  Sign up
                </Link>
              </div>
            )}
          </div>
        )}
      </div>
    </nav>
  );
}
