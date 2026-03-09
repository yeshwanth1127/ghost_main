import { Button, Header, Input, Selection, TextInput } from "@/components";
import { UseSettingsReturn } from "@/types";
import curl2Json, { ResultJSON } from "@bany/curl-to-json";
import { KeyIcon, TrashIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface OpenRouterModel {
  provider: string;
  name: string;
  id: string;
  model: string;
  description: string;
  modality: string;
  isAvailable: boolean;
}

export const Providers = ({
  allSttProviders,
  selectedSttProvider,
  onSetSelectedSttProvider,
  sttVariables,
}: UseSettingsReturn) => {
  const [localSelectedProvider, setLocalSelectedProvider] =
    useState<ResultJSON | null>(null);
  const [models, setModels] = useState<OpenRouterModel[]>([]);
  const [isModelsLoading, setIsModelsLoading] = useState(false);

  useEffect(() => {
    const loadModels = async () => {
      setIsModelsLoading(true);
      try {
        // Uses backend env-configured API access key, same flow as chat settings.
        const fetched = await invoke<OpenRouterModel[]>("fetch_models");
        setModels(Array.isArray(fetched) ? fetched : []);
      } catch (e) {
        console.error("[STT Providers] Failed to fetch OpenRouter models:", e);
        setModels([]);
      } finally {
        setIsModelsLoading(false);
      }
    };

    loadModels();
  }, []);

  useEffect(() => {
    if (selectedSttProvider?.provider) {
      const provider = allSttProviders?.find(
        (p) => p?.id === selectedSttProvider?.provider
      );
      if (provider) {
        const json = curl2Json(provider?.curl);
        setLocalSelectedProvider(json as ResultJSON);
      }
    }
  }, [selectedSttProvider?.provider]);

  const findKeyAndValue = (key: string) => {
    return sttVariables?.find((v) => v?.key === key);
  };

  const getApiKeyValue = () => {
    const apiKeyVar = findKeyAndValue("api_key");
    if (!apiKeyVar || !selectedSttProvider?.variables) return "";
    return selectedSttProvider?.variables?.[apiKeyVar.key] || "";
  };

  const isApiKeyEmpty = () => {
    return !getApiKeyValue().trim();
  };

  return (
    <div className="space-y-3">
      <div className="space-y-4 mt-2">
        {sttVariables
          ?.filter(
            (variable) => variable?.key !== findKeyAndValue("api_key")?.key
          )
          .map((variable) => {
            const getVariableValue = () => {
              if (!variable?.key || !selectedSttProvider?.variables) return "";
              return selectedSttProvider.variables[variable.key] || "";
            };

            return (
              <div className="space-y-1" key={variable?.key}>
                {variable?.key === "model" ? (
                  <>
                    <Header
                      title="OpenRouter STT Model"
                      description="Choose a model from OpenRouter (loaded with env API key)."
                    />
                    <Selection
                      selected={getVariableValue() || ""}
                      options={models.map((m) => ({
                        label: `${m.name} (${m.provider})`,
                        value: m.id,
                        isCustom: false,
                      }))}
                      placeholder={
                        isModelsLoading
                          ? "Loading OpenRouter models..."
                          : "Select OpenRouter model"
                      }
                      isLoading={isModelsLoading}
                      disableWhileLoading={false}
                      contentSide="bottom"
                      contentAlign="start"
                      contentSideOffset={8}
                      contentAvoidCollisions={false}
                      onChange={(value) => {
                        if (!variable?.key || !selectedSttProvider) return;

                        onSetSelectedSttProvider({
                          ...selectedSttProvider,
                          variables: {
                            ...selectedSttProvider.variables,
                            [variable.key]: value,
                          },
                        });
                      }}
                    />
                  </>
                ) : (
                  <>
                    <Header
                      title={variable?.value || ""}
                      description={`add your preferred ${variable?.key?.replace(
                        /_/g,
                        " "
                      )} for ${
                        allSttProviders?.find(
                          (p) => p?.id === selectedSttProvider?.provider
                        )?.isCustom
                          ? "Custom Provider"
                          : selectedSttProvider?.provider
                      }`}
                    />
                    <TextInput
                      placeholder={`Enter ${
                        allSttProviders?.find(
                          (p) => p?.id === selectedSttProvider?.provider
                        )?.isCustom
                          ? "Custom Provider"
                          : selectedSttProvider?.provider
                      } ${variable?.key?.replace(/_/g, " ") || "value"}`}
                      value={getVariableValue()}
                      onChange={(value) => {
                        if (!variable?.key || !selectedSttProvider) return;

                        onSetSelectedSttProvider({
                          ...selectedSttProvider,
                          variables: {
                            ...selectedSttProvider.variables,
                            [variable.key]: value,
                          },
                        });
                      }}
                    />
                  </>
                )}
              </div>
            );
          })}
      </div>
    </div>
  );
};
