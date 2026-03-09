import { Shield } from "lucide-react";
import { useApp } from "@/contexts";

/**
 * Admin Badge Component
 * Displays when user has super admin (owner) license activated
 */
export const AdminBadge = () => {
  const { isAdmin } = useApp();

  if (!isAdmin) {
    return null;
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-gradient-to-r from-primary/20 to-accent/20 border border-primary/30 hover:border-primary/50 transition-colors">
      <Shield className="w-3.5 h-3.5 text-primary" />
      <span className="text-xs font-semibold text-primary">SUPER ADMIN</span>
    </div>
  );
};

export default AdminBadge;
