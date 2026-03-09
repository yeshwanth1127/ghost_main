import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components";
import { Loader2, X } from "lucide-react";
import { useEffect, useState } from "react";

export const Selection = ({
  selected,
  onChange,
  options,
  placeholder,
  isLoading = false,
  disabled = false,
  disableWhileLoading = true,
  contentSide,
  contentAlign,
  contentSideOffset,
  contentAvoidCollisions,
}: {
  selected?: string;
  onChange: (value: any) => void;
  options: { label: string; value: string; isCustom?: boolean }[] | [];
  placeholder?: string;
  isLoading?: boolean;
  disabled?: boolean;
  disableWhileLoading?: boolean;
  contentSide?: "top" | "right" | "bottom" | "left";
  contentAlign?: "start" | "center" | "end";
  contentSideOffset?: number;
  contentAvoidCollisions?: boolean;
}) => {
  const [open, setOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    const handleWindowBlur = () => {
      setOpen(false);
    };

    window.addEventListener("blur", handleWindowBlur);
    return () => window.removeEventListener("blur", handleWindowBlur);
  }, []);

  const filteredOptions = options.filter((opt) =>
    opt.label.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <Select
      open={open}
      onOpenChange={setOpen}
      value={selected || ""}
      onValueChange={(value) => onChange(value)}
    >
      <SelectTrigger
        disabled={disabled || (disableWhileLoading && isLoading)}
        className="w-full h-11 border-1 border-input/50 focus:border-primary/50 transition-colors"
      >
        {isLoading ? (
          <div className="flex items-center gap-2">
            Loading... <Loader2 className="w-4 h-4 animate-spin" />
          </div>
        ) : (
          <SelectValue
            placeholder={placeholder}
            className="flex items-center gap-2"
          ></SelectValue>
        )}
      </SelectTrigger>
      <SelectContent
        side={contentSide}
        align={contentAlign}
        sideOffset={contentSideOffset}
        avoidCollisions={contentAvoidCollisions}
        className="p-0"
      >
        <div className="p-2 border-b border-input/50">
          <div className="relative">
            <input
              type="text"
              placeholder="Search..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full px-3 py-2 text-sm border border-input/50 rounded-md bg-background placeholder-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery("")}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>
        </div>
        <div className="max-h-48 overflow-y-auto">
          {filteredOptions?.filter((provider) => provider.isCustom).length > 0 && (
            <div className="border-b border-input/50 pb-2">
              <div className="px-2 py-1 text-xs font-semibold text-muted-foreground">
                Custom AI Providers
              </div>
              {filteredOptions
                ?.filter((provider) => provider.isCustom)
                .map((provider) => (
                  <SelectItem
                    key={provider.value}
                    value={provider.value}
                    className="cursor-pointer hover:bg-accent/50"
                  >
                    <span className="font-medium">{provider.label}</span>
                  </SelectItem>
                ))}
            </div>
          )}
          {filteredOptions
            ?.filter((provider) => !provider.isCustom)
            .map((provider) => (
              <SelectItem
                key={provider.value}
                value={provider.value}
                className="cursor-pointer hover:bg-accent/50"
              >
                <span className="font-medium">{provider.label}</span>
              </SelectItem>
            ))}
          {filteredOptions.length === 0 && (
            <div className="px-4 py-6 text-center text-sm text-muted-foreground">
              No results found
            </div>
          )}
        </div>
      </SelectContent>
    </Select>
  );
};
