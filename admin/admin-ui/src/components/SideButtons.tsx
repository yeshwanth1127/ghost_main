import { Link, useLocation } from "react-router-dom";

const btnClass = "w-full py-4 px-3 sm:py-5 sm:px-4 lg:py-6 lg:px-8 rounded-lg border border-white bg-black text-white text-center font-medium transition-colors hover:bg-white/5 text-xs sm:text-sm lg:text-base";
const btnStyle = { fontFamily: "Space Grotesk, sans-serif" };

export default function SideButtons() {
  const location = useLocation();
  const isAdmin = !!localStorage.getItem("admin_token");
  const isCustomer = !!localStorage.getItem("customer_token");
  const isHome = location.pathname === "/";

  return (
    <div className="fixed bottom-0 left-0 right-0 lg:left-auto lg:right-8 xl:right-16 lg:top-1/2 lg:bottom-auto lg:-translate-y-1/2 grid grid-cols-2 lg:grid-cols-1 gap-2 sm:gap-3 lg:gap-6 p-3 sm:p-4 lg:p-0 w-full lg:w-56 z-40 lg:bg-transparent bg-black/95 border-t lg:border-t-0 border-white/10">
      {!isHome && (
        <Link to="/" className={btnClass} style={btnStyle}>
          Home
        </Link>
      )}
      <Link to="/download" className={btnClass} style={btnStyle}>
        Download
      </Link>
      <Link to="/subscriptions" className={btnClass} style={btnStyle}>
        Subscriptions
      </Link>
      <Link to="/demo" className={btnClass} style={btnStyle}>
        Watch Demo
      </Link>
      {isAdmin ? (
        <Link to="/dashboard" className={btnClass} style={btnStyle}>
          Admin Dashboard
        </Link>
      ) : isCustomer ? (
        <Link to="/account" className={btnClass} style={btnStyle}>
          My Account
        </Link>
      ) : (
        <Link to="/login" className={btnClass} style={btnStyle}>
          Login / Sign Up
        </Link>
      )}
    </div>
  );
}
