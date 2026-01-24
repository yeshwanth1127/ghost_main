import {
  ChangeEvent,
  FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Paperclip, Trash2 } from "lucide-react";
import { Button, Header, Input, Label, Textarea } from "@/components";
import { AttachedFile } from "@/types";
import { MAX_FILES } from "@/config";

const LEAVE_FORM_DEFAULTS = {
  name: "",
  usn: "",
  department: "",
  reason: "",
} as const;

type LeaveApplicationFormValues = typeof LEAVE_FORM_DEFAULTS;

interface LeaveApplicationEventDetail {
  formValues: LeaveApplicationFormValues;
  attachments: AttachedFile[];
}

export const ApplyForLeave = () => {
  const [formValues, setFormValues] = useState<LeaveApplicationFormValues>(
    () => ({ ...LEAVE_FORM_DEFAULTS })
  );
  const [attachedFiles, setAttachedFiles] = useState<AttachedFile[]>([]);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const successTimeoutRef = useRef<number | null>(null);
  const [showSuccess, setShowSuccess] = useState(false);

  const remainingSlots = useMemo(
    () => Math.max(0, MAX_FILES - attachedFiles.length),
    [attachedFiles.length]
  );

  const handleInputChange = useCallback(
    (field: keyof typeof formValues, value: string) => {
      setFormValues((prev) => ({ ...prev, [field]: value }));
    },
    []
  );

  const fileToBase64 = useCallback((file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      reader.onload = () => {
        const base64 = (reader.result as string)?.split(",")[1] || "";
        resolve(base64);
      };
      reader.onerror = reject;
    });
  }, []);

  const createAttachedFile = useCallback(
    async (file: File): Promise<AttachedFile> => {
      const base64 = await fileToBase64(file);
      return {
        id: crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`,
        name: file.name,
        type: file.type,
        base64,
        size: file.size,
      };
    },
    [fileToBase64]
  );

  const handleFileSelect = useCallback(
    async (event: ChangeEvent<HTMLInputElement>) => {
      const files = Array.from(event.target.files ?? []);
      event.target.value = "";

      if (files.length === 0) {
        return;
      }

      if (remainingSlots <= 0) {
        setUploadError(`You can only upload ${MAX_FILES} images.`);
        return;
      }

      const validImages = files.filter((file) => file.type.startsWith("image/"));

      if (validImages.length !== files.length) {
        setUploadError("Only image attachments are supported right now.");
      } else {
        setUploadError(null);
      }

      const limitedImages = validImages.slice(0, remainingSlots);

      if (limitedImages.length === 0) {
        return;
      }

      try {
        const processed = await Promise.all(
          limitedImages.map((file) => createAttachedFile(file))
        );
        setAttachedFiles((prev) => [...prev, ...processed]);
      } catch (error) {
        console.error("Failed to process attachment:", error);
        setUploadError("We couldn't process one of the files. Please try again.");
      }
    },
    [createAttachedFile, remainingSlots]
  );

  const removeFile = useCallback((fileId: string) => {
    setAttachedFiles((prev) => prev.filter((file) => file.id !== fileId));
    setUploadError(null);
  }, []);

  const clearFiles = useCallback(() => {
    setAttachedFiles([]);
    setUploadError(null);
  }, []);

  const openFileDialog = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const formatFileSize = useCallback((size: number) => {
    if (size >= 1024 * 1024) {
      return `${(size / 1024 / 1024).toFixed(2)} MB`;
    }
    return `${(size / 1024).toFixed(1)} KB`;
  }, []);

  const handleSubmit = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();

      const detail: LeaveApplicationEventDetail = {
        formValues: { ...formValues },
        attachments: attachedFiles.map((file) => ({ ...file })),
      };

      if (typeof window !== "undefined") {
        window.dispatchEvent(
          new CustomEvent<LeaveApplicationEventDetail>(
            "leaveApplicationSubmitted",
            {
              detail,
            }
          )
        );
      }

      setFormValues(() => ({ ...LEAVE_FORM_DEFAULTS }));
      setAttachedFiles([]);
      setUploadError(null);
      setShowSuccess(true);

      if (successTimeoutRef.current) {
        window.clearTimeout(successTimeoutRef.current);
      }

      successTimeoutRef.current = window.setTimeout(() => {
        setShowSuccess(false);
        successTimeoutRef.current = null;
      }, 4000);
    },
    [attachedFiles, formValues]
  );

  useEffect(() => {
    return () => {
      if (successTimeoutRef.current) {
        window.clearTimeout(successTimeoutRef.current);
      }
    };
  }, []);

  return (
    <div id="apply-for-leave" className="space-y-3">
      <Header
        title="Apply for leave"
        description="Submit your time-off request directly from Scribe."
        isMainTitle
      />

      <form className="space-y-5" onSubmit={handleSubmit}>
        {showSuccess && (
          <div className="rounded-md border border-green-200 bg-green-50 px-3 py-2 text-xs text-green-700">
            Leave request sent for AI analysis. You can view the summary in the
            response panel.
          </div>
        )}

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div className="space-y-1.5">
            <Label htmlFor="leave-name">Full name</Label>
            <Input
              id="leave-name"
              placeholder="Enter your full name"
              value={formValues.name}
              onChange={(event) => handleInputChange("name", event.target.value)}
              required
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="leave-usn">USN</Label>
            <Input
              id="leave-usn"
              placeholder="Enter your USN"
              value={formValues.usn}
              onChange={(event) => handleInputChange("usn", event.target.value)}
              required
            />
          </div>

          <div className="space-y-1.5 md:col-span-2">
            <Label htmlFor="leave-department">Department</Label>
            <Input
              id="leave-department"
              placeholder="e.g. Product, Engineering, HR"
              value={formValues.department}
              onChange={(event) =>
                handleInputChange("department", event.target.value)
              }
              required
            />
          </div>
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="leave-reason">Reason</Label>
          <Textarea
            id="leave-reason"
            placeholder="Share the context, dates, and any additional information."
            value={formValues.reason}
            onChange={(event) =>
              handleInputChange("reason", event.target.value)
            }
            className="min-h-[120px] resize-y"
            required
          />
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="space-y-0.5">
              <Label>Attachments</Label>
              <p className="text-xs text-muted-foreground">
                Add supporting documents or screenshots (up to {MAX_FILES} images).
              </p>
            </div>

            <div className="flex items-center gap-2">
              {attachedFiles.length > 0 && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={clearFiles}
                  className="h-8 px-2 text-xs"
                >
                  Clear all
                </Button>
              )}
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={openFileDialog}
                className="h-8"
              >
                <Paperclip className="mr-2 h-3.5 w-3.5" />
                Upload
              </Button>
            </div>
          </div>

          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            multiple
            onChange={handleFileSelect}
            className="hidden"
          />

          {uploadError && (
            <p className="text-xs text-destructive">{uploadError}</p>
          )}

          {attachedFiles.length > 0 && (
            <div className="grid gap-3 sm:grid-cols-2">
              {attachedFiles.map((file) => (
                <div
                  key={file.id}
                  className="relative overflow-hidden rounded-lg border border-border/80 bg-muted/40"
                >
                  <div className="aspect-video bg-background/40">
                    <img
                      src={`data:${file.type};base64,${file.base64}`}
                      alt={file.name}
                      className="h-full w-full object-cover"
                    />
                  </div>
                  <div className="flex items-center justify-between gap-2 border-t px-3 py-2">
                    <div className="min-w-0">
                      <p className="truncate text-xs font-medium">{file.name}</p>
                      <p className="text-[11px] text-muted-foreground">
                        {formatFileSize(file.size)}
                      </p>
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => removeFile(file.id)}
                      title="Remove attachment"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}

          {attachedFiles.length === 0 && (
            <div className="rounded-lg border border-dashed border-border/70 p-6 text-sm text-muted-foreground">
              No attachments yet. You can add up to {MAX_FILES} image files.
            </div>
          )}

          {remainingSlots < MAX_FILES && remainingSlots > 0 && (
            <p className="text-xs text-muted-foreground">
              {remainingSlots} attachment{remainingSlots === 1 ? "" : "s"} left.
            </p>
          )}
        </div>

        <div className="flex justify-end">
          <Button type="submit" className="w-full md:w-auto">
            Submit leave request
          </Button>
        </div>
      </form>
    </div>
  );
};

