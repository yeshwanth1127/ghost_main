import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { FolderOpen } from "lucide-react";

interface PathPromptDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  actionType: string;
  onConfirm: (path: string) => void;
  isDirectory?: boolean;
  isDestination?: boolean;
}

export function PathPromptDialog({
  open: isOpen,
  onOpenChange,
  actionType,
  onConfirm,
  isDirectory = false,
  isDestination = false,
}: PathPromptDialogProps) {
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleBrowse = async () => {
    // Browse functionality requires @tauri-apps/plugin-dialog
    // For now, the button is disabled and users should type paths manually
    // TODO: Add @tauri-apps/plugin-dialog package to enable file browser
    console.log("Browse functionality requires @tauri-apps/plugin-dialog package");
  };

  const handleConfirm = () => {
    if (!path.trim()) {
      setError("Path is required");
      return;
    }
    onConfirm(path.trim());
    setPath("");
    setError(null);
    onOpenChange(false);
  };

  const handleCancel = () => {
    setPath("");
    setError(null);
    onOpenChange(false);
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {isDestination ? "Select Destination Path" : "Select File Path"}
          </DialogTitle>
          <DialogDescription>
            Please provide the {isDirectory ? "directory" : "file"} path for{" "}
            {actionType}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="path-input">
              {isDestination ? "Destination" : "Path"}
            </Label>
            <div className="flex gap-2">
              <Input
                id="path-input"
                value={path}
                onChange={(e) => {
                  setPath(e.target.value);
                  setError(null);
                }}
                placeholder={
                  isDirectory
                    ? "/path/to/directory"
                    : "/path/to/file.txt"
                }
                className="flex-1"
              />
              <Button
                type="button"
                variant="outline"
                onClick={handleBrowse}
                disabled={true}
                title="Browse requires @tauri-apps/plugin-dialog (type path manually)"
              >
                <FolderOpen className="w-4 h-4 opacity-50" />
              </Button>
            </div>
            {error && (
              <p className="text-sm text-red-600">{error}</p>
            )}
            <p className="text-xs text-muted-foreground">
              Type or paste an absolute path (e.g., C:\Users\Name\file.txt or /home/user/file.txt)
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={handleCancel}>
            Cancel
          </Button>
          <Button onClick={handleConfirm}>Confirm</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

