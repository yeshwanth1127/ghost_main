import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import Home from "./pages/Home";
import Login from "./pages/Login";
import SignUp from "./pages/SignUp";
import Dashboard from "./pages/Dashboard";
import Subscriptions from "./pages/Subscriptions";
import PaySuccess from "./pages/PaySuccess";
import Account from "./pages/Account";
import Download from "./pages/Download";

function AdminProtectedRoute({ children }: { children: React.ReactNode }) {
  const token = localStorage.getItem("admin_token");
  if (!token) {
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}

function CustomerProtectedRoute({ children }: { children: React.ReactNode }) {
  const token = localStorage.getItem("customer_token");
  if (!token) {
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Home />} />
          <Route path="/login" element={<Login />} />
          <Route path="/signup" element={<SignUp />} />
          <Route path="/subscriptions" element={<Subscriptions />} />
          <Route path="/download" element={<Download />} />
          <Route path="/pay/success" element={<PaySuccess />} />
          <Route
            path="/account"
            element={
              <CustomerProtectedRoute>
                <Account />
              </CustomerProtectedRoute>
            }
          />
          <Route
            path="/dashboard"
            element={
              <AdminProtectedRoute>
                <Dashboard />
              </AdminProtectedRoute>
            }
          />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
