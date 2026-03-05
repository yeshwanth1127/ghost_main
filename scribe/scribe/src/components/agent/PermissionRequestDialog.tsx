import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Shield, X, AlertTriangle, CheckCircle } from "lucide-react";

/** Canonical intent (Moltbot-style) for permissions/audit */
export interface CanonicalIntent {
  human_readable?: string;
  goal_alignment?: string;
  irreversible?: boolean;
  risk_factors?: string[];
}

interface PermissionRequestDialogProps {
  runId: string;
  permissionId: string;
  capability: string;
  scope: string;
  reason: string;
  riskScore: number;
  /** When present, show human-readable intent and risk factors from execution ticket */
  canonicalIntent?: CanonicalIntent;
  onClose: () => void;
}

export const PermissionRequestDialog: React.FC<PermissionRequestDialogProps> = ({
  runId,
  permissionId,
  capability,
  scope,
  reason,
  riskScore,
  canonicalIntent,
  onClose,
}) => {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handlePermissionReply = async (granted: boolean) => {
    if (!permissionId?.trim()) {
      setError("Permission ID is missing. Please try again.");
      return;
    }
    setError(null);
    setIsSubmitting(true);
    try {
      await invoke("reply_permission", {
        runId,
        permissionId,
        granted,
      });
      onClose();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("Failed to reply to permission:", err);
      setError(msg || "Failed to send permission. Check console for details.");
      setIsSubmitting(false);
    }
  };

  const getRiskLevel = (score: number): { label: string; color: string; icon: React.ReactNode } => {
    if (score >= 0.8) {
      return {
        label: "Critical Risk",
        color: "text-red-600 dark:text-red-400",
        icon: <AlertTriangle className="w-4 h-4" />,
      };
    } else if (score >= 0.5) {
      return {
        label: "Medium Risk",
        color: "text-yellow-600 dark:text-yellow-400",
        icon: <AlertTriangle className="w-4 h-4" />,
      };
    } else if (score >= 0.2) {
      return {
        label: "Low Risk",
        color: "text-blue-600 dark:text-blue-400",
        icon: <CheckCircle className="w-4 h-4" />,
      };
    } else {
      return {
        label: "Very Low Risk",
        color: "text-green-600 dark:text-green-400",
        icon: <CheckCircle className="w-4 h-4" />,
      };
    }
  };

  const risk = getRiskLevel(riskScore);

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="w-full max-w-lg m-4 p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Shield className="w-5 h-5 text-blue-600" />
            <h3 className="text-lg font-semibold">Permission Request</h3>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="h-8 w-8 p-0"
            disabled={isSubmitting}
          >
            <X className="w-4 h-4" />
          </Button>
        </div>

        <div className="space-y-4">
          {canonicalIntent?.human_readable && (
            <div>
              <Label className="text-sm font-medium text-muted-foreground">Action</Label>
              <p className="text-sm mt-1">{canonicalIntent.human_readable}</p>
            </div>
          )}
          <div>
            <Label className="text-sm font-medium text-muted-foreground">Capability</Label>
            <p className="text-sm font-mono mt-1">{capability}</p>
          </div>

          <div>
            <Label className="text-sm font-medium text-muted-foreground">Reason</Label>
            <p className="text-sm mt-1">{reason}</p>
          </div>
          {canonicalIntent?.goal_alignment && (
            <div>
              <Label className="text-sm font-medium text-muted-foreground">Goal alignment</Label>
              <p className="text-sm mt-1">{canonicalIntent.goal_alignment}</p>
            </div>
          )}
          {canonicalIntent?.risk_factors && canonicalIntent.risk_factors.length > 0 && (
            <div>
              <Label className="text-sm font-medium text-muted-foreground">Risk factors</Label>
              <p className="text-sm mt-1">{canonicalIntent.risk_factors.join(", ")}</p>
            </div>
          )}
          {canonicalIntent?.irreversible && (
            <p className="text-sm text-amber-600 dark:text-amber-400">This action cannot be undone.</p>
          )}

          <div>
            <Label className="text-sm font-medium text-muted-foreground">Scope</Label>
            <p className="text-sm mt-1">{scope}</p>
          </div>

          {error && (
            <div className="rounded-md bg-destructive/15 p-3 text-sm text-destructive">
              {error}
            </div>
          )}
          <div>
            <Label className="text-sm font-medium text-muted-foreground">Risk Level</Label>
            <div className={`flex items-center gap-2 mt-1 ${risk.color}`}>
              {risk.icon}
              <span className="text-sm font-medium">{risk.label}</span>
              <span className="text-xs text-muted-foreground">
                (Score: {(riskScore * 100).toFixed(0)}%)
              </span>
            </div>
          </div>

          <div className="pt-4 border-t">
            <div className="flex gap-2 justify-end">
              <Button
                variant="outline"
                onClick={() => handlePermissionReply(false)}
                disabled={isSubmitting}
              >
                Deny
              </Button>
              <Button
                onClick={() => handlePermissionReply(true)}
                disabled={isSubmitting}
              >
                {isSubmitting ? "Processing..." : "Allow"}
              </Button>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
};
