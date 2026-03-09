import { AlertCircle } from "lucide-react";
import { useApp } from "@/contexts";

/**
 * Admin Notice Component
 * Shows a clean notice when user has admin access activated
 * Can be placed in settings or dashboard
 */
export const AdminNotice = () => {
  const { isAdmin } = useApp();

  if (!isAdmin) {
    return null;
  }

  return (
    <div className="w-full p-4 rounded-lg bg-gradient-to-r from-orange-950/40 to-rose-950/40 border border-primary/30 space-y-3">
      <div className="flex items-start gap-3">
        <AlertCircle className="w-5 h-5 text-primary flex-shrink-0 mt-0.5" />
        <div className="space-y-1">
          <h3 className="font-semibold text-primary text-sm">
            Super Admin Access Active
          </h3>
          <p className="text-xs text-primary/80 leading-relaxed">
            You are currently running Ghost with <span className="font-medium">super admin privileges</span>.
            Unlimited tokens, unlimited instances, and no usage restrictions apply.
          </p>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-2 pt-2 border-t border-primary/20">
        <div className="flex flex-col items-center py-2">
          <span className="text-2xl font-bold text-primary">∞</span>
          <span className="text-xs text-primary/60 mt-1">Tokens</span>
        </div>
        <div className="flex flex-col items-center py-2">
          <span className="text-2xl font-bold text-primary">∞</span>
          <span className="text-xs text-primary/60 mt-1">Instances</span>
        </div>
        <div className="flex flex-col items-center py-2">
          <span className="text-2xl font-bold text-primary">∞</span>
          <span className="text-xs text-primary/60 mt-1">Expiry</span>
        </div>
      </div>
    </div>
  );
};

export default AdminNotice;
