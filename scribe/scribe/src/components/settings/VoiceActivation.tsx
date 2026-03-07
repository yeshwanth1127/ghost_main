import { Switch, Label, Header, Input } from "@/components";
import { useApp } from "@/contexts";

export const VoiceActivation = () => {
  const { voiceActivation, updateVoiceActivation } = useApp();

  return (
    <div id="voice-activation" className="space-y-4">
      <Header
        title="Voice activation"
        description="Collapse to floating logo after inactivity; expand with click or voice command"
        isMainTitle
      />

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <Label className="text-sm font-medium">Auto-collapse (25s inactivity)</Label>
            <p className="text-xs text-muted-foreground">
              Collapse to floating logo when idle for 25 seconds
            </p>
          </div>
          <Switch
            checked={voiceActivation.autoCollapseEnabled}
            onCheckedChange={(checked) =>
              updateVoiceActivation({ autoCollapseEnabled: checked })
            }
            aria-label="Toggle auto-collapse"
          />
        </div>

        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <Label className="text-sm font-medium">Voice command to expand</Label>
            <p className="text-xs text-muted-foreground">
              Say &quot;Ghost&quot; or &quot;Hey Ghost&quot; when collapsed to expand
            </p>
          </div>
          <Switch
            checked={voiceActivation.enabled}
            onCheckedChange={(checked) => updateVoiceActivation({ enabled: checked })}
            aria-label="Toggle voice activation"
          />
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium">Custom wake phrase (optional)</Label>
          <p className="text-xs text-muted-foreground">
            Add a custom phrase like &quot;open ghost&quot; or &quot;expand&quot;
          </p>
          <Input
            type="text"
            placeholder="e.g. open ghost, expand"
            value={voiceActivation.customPhrase}
            onChange={(e) =>
              updateVoiceActivation({ customPhrase: e.target.value })
            }
            className="max-w-xs"
          />
        </div>
      </div>
    </div>
  );
};
