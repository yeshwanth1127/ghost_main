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
    <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-gradient-to-r from-purple-500/20 to-pink-500/20 border border-purple-500/30 hover:border-purple-500/50 transition-colors">
      <Shield className="w-3.5 h-3.5 text-purple-400" />
      <span className="text-xs font-semibold text-purple-300">SUPER ADMIN</span>
    </div>
  );
};

export default AdminBadge;
