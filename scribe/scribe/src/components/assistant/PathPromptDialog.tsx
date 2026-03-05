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
    try {
      const { open, save } = await import("@tauri-apps/plugin-dialog");
      let selected: string | string[] | null = null;
      if (isDestination && !isDirectory) {
        selected = await save({
          title: "Choose file location",
          defaultPath: path.trim() || undefined,
        });
      } else {
        selected = await open({
          title: isDirectory ? "Select directory" : "Select file",
          directory: isDirectory,
          multiple: false,
          defaultPath: path.trim() || undefined,
        });
      }
      if (selected) {
        const resolved = typeof selected === "string" ? selected : Array.isArray(selected) ? selected[0] : null;
        if (resolved) {
          setPath(resolved);
          setError(null);
        }
      }
    } catch (err) {
      console.error("File picker failed:", err);
      setError("File picker failed. Please enter the path manually.");
    }
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
                title={isDirectory ? "Browse for directory" : "Browse for file"}
              >
                <FolderOpen className="w-4 h-4" />
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

