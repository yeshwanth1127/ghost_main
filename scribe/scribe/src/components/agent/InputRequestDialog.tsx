import { useState, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { FileText, X, FolderOpen } from "lucide-react";

function isTauri(): boolean {
  return typeof window !== "undefined" && !!(window as unknown as { __TAURI__?: unknown }).__TAURI__;
}

interface InputRequestDialogProps {
  runId: string;
  inputRequestId: string;
  capability: string;
  intent: string;
  missingFields: string[];
  schema: any;
  currentInputs: any;
  onClose: () => void;
}

export const InputRequestDialog: React.FC<InputRequestDialogProps> = ({
  runId,
  inputRequestId,
  capability,
  intent,
  missingFields,
  schema,
  currentInputs,
  onClose,
}) => {
  const [formData, setFormData] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const pathPickerOpenedRef = useRef(false);
  const initializedForRequestRef = useRef<string | null>(null);

  // Initialize form data from current inputs only once per input request (so parent re-renders don't wipe typed content)
  useEffect(() => {
    if (initializedForRequestRef.current === inputRequestId) return;
    initializedForRequestRef.current = inputRequestId;
    const initial: Record<string, string> = {};
    if (currentInputs && typeof currentInputs === 'object') {
      for (const [key, value] of Object.entries(currentInputs)) {
        if (typeof value === 'string') {
          initial[key] = value;
        }
      }
    }
    setFormData(initial);
  }, [inputRequestId, currentInputs]);

  // When path is missing on desktop, open file/folder picker automatically
  useEffect(() => {
    if (!missingFields.includes("path") || !isTauri() || pathPickerOpenedRef.current) return;
    pathPickerOpenedRef.current = true;
    const t = setTimeout(async () => {
      try {
        const { save, open } = await import("@tauri-apps/plugin-dialog");
        const isWrite = capability.includes("write");
        const selected = isWrite
          ? await save({ title: "Choose file location", defaultPath: formData.path || undefined })
          : await open({ title: "Select file", multiple: false });
        if (selected) {
          const path = typeof selected === "string" ? selected : Array.isArray(selected) ? selected[0] : null;
          if (path) setFormData((prev) => ({ ...prev, path }));
        }
      } catch (_) {
        pathPickerOpenedRef.current = false;
      }
    }, 350);
    return () => clearTimeout(t);
  }, [capability, missingFields]);

  const handleSubmit = async () => {
    setIsSubmitting(true);
    try {
      // Build inputs object from form data
      const inputs: Record<string, any> = {};
      
      // Get schema properties to determine types
      const properties = schema?.properties || {};
      
      for (const field of missingFields) {
        const value = formData[field] || '';
        const fieldSchema = properties[field];
        
        // Convert to appropriate type based on schema
        if (fieldSchema?.type === 'number') {
          inputs[field] = value ? parseFloat(value) : null;
        } else if (fieldSchema?.type === 'boolean') {
          inputs[field] = value === 'true' || value === '1';
        } else {
          inputs[field] = value;
        }
      }
      
      // Include any existing inputs that aren't missing
      if (currentInputs && typeof currentInputs === 'object') {
        for (const [key, value] of Object.entries(currentInputs)) {
          if (!missingFields.includes(key)) {
            inputs[key] = value;
          }
        }
      }
      
      await invoke("reply_input", {
        runId,
        inputRequestId,
        inputs,
      });
      
      onClose();
    } catch (error) {
      console.error("Failed to submit inputs:", error);
      setIsSubmitting(false);
    }
  };

  const getFieldDescription = (fieldName: string): string => {
    const properties = schema?.properties || {};
    const fieldSchema = properties[fieldName];
    return fieldSchema?.description || `Enter ${fieldName}`;
  };

  const getFieldType = (fieldName: string): string => {
    const properties = schema?.properties || {};
    const fieldSchema = properties[fieldName];
    return fieldSchema?.type || 'string';
  };

  const isFieldRequired = (fieldName: string): boolean => {
    const required = schema?.required || [];
    return required.includes(fieldName);
  };

  const handleBrowsePath = async (field: string) => {
    try {
      const { save, open } = await import("@tauri-apps/plugin-dialog");
      const isWrite = capability.includes("write");
      const selected = isWrite
        ? await save({
            title: "Choose file location",
            defaultPath: formData[field] || undefined,
          })
        : await open({
            title: "Select file",
            defaultPath: formData[field] || undefined,
            multiple: false,
          });
      
      // Handle the selected path
      if (selected) {
        if (typeof selected === 'string') {
          setFormData({ ...formData, [field]: selected });
        } else if (Array.isArray(selected) && selected.length > 0) {
          setFormData({ ...formData, [field]: selected[0] });
        }
      }
    } catch (error) {
      console.error("Failed to open file picker:", error);
      // Fallback: user can still type the path manually
      alert("File picker failed. Please enter the path manually.");
    }
  };

  const handleCancelClick = async () => {
    const confirmed = isTauri()
      ? await (async () => {
          const { confirm } = await import("@tauri-apps/plugin-dialog");
          return confirm("Cancel this input request? The operation will be cancelled.", {
            title: "Cancel input",
            kind: "warning",
          });
        })()
      : window.confirm("Cancel this input request? The operation will be cancelled.");
    if (!confirmed) return;
    try {
      if (isTauri()) {
        await invoke("cancel_input_request", { inputRequestId });
      }
    } catch (e) {
      console.error("Failed to cancel input request:", e);
    }
    onClose();
  };

  const dialogContent = (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-[9999]"
      style={{ pointerEvents: "auto" }}
      role="dialog"
      aria-modal="true"
      aria-labelledby="input-required-title"
    >
      <Card
        className="w-full max-w-2xl max-h-[90vh] overflow-auto m-4 p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <FileText className="w-5 h-5 text-blue-600" />
            <h3 id="input-required-title" className="text-lg font-semibold">Input Required</h3>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="h-8 w-8 p-0"
          >
            <X className="w-4 h-4" />
          </Button>
        </div>

        <div className="space-y-4">
          <div>
            <p className="text-sm text-muted-foreground mb-1">
              <span className="font-medium">Capability:</span> {capability}
            </p>
            {intent && (
              <p className="text-sm text-muted-foreground mb-4">
                <span className="font-medium">Intent:</span> {intent}
              </p>
            )}
          </div>

          <div className="space-y-4">
            {missingFields.map((field) => {
              const fieldType = getFieldType(field);
              const isRequired = isFieldRequired(field);
              const description = getFieldDescription(field);
              
              return (
                <div key={field} className="space-y-2">
                  <Label htmlFor={field}>
                    {field}
                    {isRequired && <span className="text-red-500 ml-1">*</span>}
                  </Label>
                  {fieldType === 'string' && field.toLowerCase().includes('content') ? (
                    <Textarea
                      id={field}
                      value={formData[field] || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, [field]: e.target.value })
                      }
                      placeholder={description}
                      rows={6}
                      className="font-mono text-sm"
                    />
                  ) : fieldType === 'string' && field.toLowerCase().includes('path') ? (
                    <div className="space-y-2">
                      <div className="flex gap-2">
                        <Input
                          id={field}
                          type="text"
                          value={formData[field] || ''}
                          onChange={(e) =>
                            setFormData({ ...formData, [field]: e.target.value })
                          }
                          placeholder={description || "Enter file path or click browse to choose"}
                          className="font-mono text-sm flex-1"
                        />
                        <Button
                          type="button"
                          variant="outline"
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            void handleBrowsePath(field);
                          }}
                          className="shrink-0 relative z-10"
                          title="Browse for file location"
                          tabIndex={0}
                        >
                          <FolderOpen className="w-4 h-4" />
                        </Button>
                      </div>
                      <p className="text-xs text-muted-foreground">
                        Enter path manually or click browse to choose file location
                      </p>
                    </div>
                  ) : (
                    <Input
                      id={field}
                      type={fieldType === 'number' ? 'number' : 'text'}
                      value={formData[field] || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, [field]: e.target.value })
                      }
                      placeholder={description}
                      className="font-mono text-sm"
                    />
                  )}
                  {description && (
                    <p className="text-xs text-muted-foreground">{description}</p>
                  )}
                </div>
              );
            })}
          </div>

          <div className="flex gap-2 justify-end pt-4 border-t">
            <Button
              type="button"
              variant="outline"
              onClick={handleCancelClick}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={
                isSubmitting ||
                missingFields.some(
                  (field) =>
                    isFieldRequired(field) &&
                    (!formData[field] || formData[field].trim() === '')
                )
              }
            >
              {isSubmitting ? "Submitting..." : "Submit"}
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );

  return typeof document !== "undefined"
    ? createPortal(dialogContent, document.body)
    : dialogContent;
};
