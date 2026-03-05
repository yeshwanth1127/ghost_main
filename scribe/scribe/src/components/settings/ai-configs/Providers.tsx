import { Button, Header, Input, Selection, TextInput } from "@/components";
import { UseSettingsReturn } from "@/types";
import curl2Json, { ResultJSON } from "@bany/curl-to-json";
import { KeyIcon, TrashIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface FreeModel {
  provider: string;
  name: string;
  id: string;
  model: string;
  description: string;
  modality: string;
  isAvailable: boolean;
}

const SELECTED_SCRIBE_MODEL_KEY = "selected_Scribe_model"; // must match ScribeApiSetup

export const Providers = ({
  allAiProviders,
  selectedAIProvider,
  onSetSelectedAIProvider,
  variables,
}: UseSettingsReturn) => {
  const [localSelectedProvider, setLocalSelectedProvider] =
    useState<ResultJSON | null>(null);
  const [models, setModels] = useState<FreeModel[]>([]);
  const [isModelsLoading, setIsModelsLoading] = useState(false);
  const [selectedFreeModel, setSelectedFreeModel] = useState<FreeModel | null>(null);

  const isExoraSelected =
    selectedAIProvider?.provider === "exora" || selectedAIProvider?.provider === "ollama";

  const isGpt4oMiniSelected = selectedAIProvider?.provider === "gpt-4o-mini";

  // When GPT 4o Mini is selected, save model for Scribe API
  useEffect(() => {
    if (isGpt4oMiniSelected) {
      const model = {
        provider: "openai",
        model: "gpt-4o-mini",
        name: "GPT 4o Mini",
        id: "openai/gpt-4o-mini",
        description: "",
        modality: "text",
        isAvailable: true,
      };
      invoke("secure_storage_save", {
        items: [{ key: SELECTED_SCRIBE_MODEL_KEY, value: JSON.stringify(model) }],
      }).catch((e) => console.error("[Providers] Failed to save GPT 4o Mini model:", e));
    }
  }, [isGpt4oMiniSelected]);

  useEffect(() => {
    if (isExoraSelected) {
      const loadModels = async () => {
        setIsModelsLoading(true);
        try {
          const fetched = await invoke<FreeModel[]>("fetch_models");
          setModels(Array.isArray(fetched) ? fetched : []);
        } catch (e) {
          console.error("[Providers] Failed to fetch models:", e);
          setModels([]);
        } finally {
          setIsModelsLoading(false);
        }
      };
      loadModels();
    } else {
      setModels([]);
    }
  }, [isExoraSelected]);

  useEffect(() => {
    if (isExoraSelected) {
      invoke<{ selected_Scribe_model?: string }>("secure_storage_get")
        .then((s: { selected_Scribe_model?: string }) => {
          if (s?.selected_Scribe_model) {
            try {
              setSelectedFreeModel(JSON.parse(s.selected_Scribe_model));
            } catch {
              setSelectedFreeModel(null);
            }
          } else {
            setSelectedFreeModel(null);
          }
        })
        .catch(() => setSelectedFreeModel(null));
    } else {
      setSelectedFreeModel(null);
    }
  }, [isExoraSelected]);

  const handleFreeModelSelect = async (model: FreeModel) => {
    setSelectedFreeModel(model);
    try {
      await invoke("secure_storage_save", {
        items: [{ key: SELECTED_SCRIBE_MODEL_KEY, value: JSON.stringify(model) }],
      });
    } catch (e) {
      console.error("[Providers] Failed to save free model:", e);
    }
  };

  useEffect(() => {
    if (selectedAIProvider?.provider) {
      const provider = allAiProviders?.find(
        (p) => p?.id === selectedAIProvider?.provider
      );
      if (provider) {
        try {
          const json = curl2Json(provider?.curl);
          setLocalSelectedProvider(json as ResultJSON);
        } catch (e) {
          console.error("[Providers] Error parsing provider curl:", provider?.id, e);
          setLocalSelectedProvider(null);
        }
      }
    }
  }, [selectedAIProvider?.provider, allAiProviders]);

  const findKeyAndValue = (key: string) => {
    return variables?.find((v) => v?.key === key);
  };

  const getApiKeyValue = () => {
    const apiKeyVar = findKeyAndValue("api_key");
    if (!apiKeyVar || !selectedAIProvider?.variables) return "";
    return selectedAIProvider?.variables?.[apiKeyVar.key] || "";
  };

  const isApiKeyEmpty = () => {
    return !getApiKeyValue().trim();
  };

  // Helper function to get display name for provider
  const getProviderDisplayName = (providerId: string | undefined): string => {
    if (!providerId) return "Custom Provider";
    if (providerId === "exora") return "Exora AI";
    if (providerId === "gpt-4o-mini") return "GPT 4o Mini";
    return providerId;
  };

  return (
    <div className="space-y-3">
      <div className="space-y-2">
        <Header
          title="Select AI Provider"
          description="Select your preferred AI service provider or custom providers to get started."
        />
        <Selection
          selected={selectedAIProvider?.provider}
          options={allAiProviders?.map((provider) => {
            try {
              const json = provider?.isCustom ? curl2Json(provider?.curl) : null;
              const label = provider?.isCustom
                ? json?.url || "Custom Provider"
                : getProviderDisplayName(provider?.id);
              return {
                label,
                value: provider?.id || "Custom Provider",
                isCustom: provider?.isCustom || false,
              };
            } catch (e) {
              console.error("[Providers] Error processing provider:", provider?.id, e);
              // Fallback: still create the option even if curl parsing fails
              return {
                label: provider?.isCustom
                  ? "Custom Provider"
                  : getProviderDisplayName(provider?.id),
                value: provider?.id || "Custom Provider",
                isCustom: provider?.isCustom || false,
              };
            }
          }).filter(Boolean) || []}
          placeholder="Choose your AI provider"
          onChange={(value) => {
            onSetSelectedAIProvider({
              provider: value,
              variables: {},
            });
          }}
        />
      </div>

      {isExoraSelected && (
        <div className="space-y-2">
          <Header
            title="Or select a model"
            description="Choose from OpenRouter models (requires Ghost API license)."
          />
          <Selection
            selected={selectedFreeModel?.id ?? "__none__"}
            options={[
              { label: "Use Exora AI only", value: "__none__", isCustom: false },
              ...models.map((m) => ({
                label: `${m.name} (${m.provider})`,
                value: m.id,
                isCustom: false,
              })),
            ]}
            placeholder={isModelsLoading ? "Loading..." : "Select model"}
            isLoading={isModelsLoading}
            onChange={(value) => {
              if (value === "__none__") {
                setSelectedFreeModel(null);
                invoke("secure_storage_remove", {
                  keys: [SELECTED_SCRIBE_MODEL_KEY],
                }).catch(() => {});
              } else {
                const model = models.find((m) => m.id === value);
                if (model) handleFreeModelSelect(model);
              }
            }}
          />
        </div>
      )}

      {localSelectedProvider ? (
        <Header
          title={`Method: ${
            localSelectedProvider?.method || "Invalid"
          }, Endpoint: ${localSelectedProvider?.url || "Invalid"}`}
          description={`If you want to use different url or method, you can always create a custom provider.`}
        />
      ) : null}

      {findKeyAndValue("api_key") ? (
        <div className="space-y-2">
          <Header
            title="API Key"
            description={`Enter your ${
              allAiProviders?.find(
                (p) => p?.id === selectedAIProvider?.provider
              )?.isCustom
                ? "Custom Provider"
                : getProviderDisplayName(selectedAIProvider?.provider)
            } API key to authenticate and access AI models. Your key is stored locally and never shared.`}
          />

          <div className="space-y-2">
            <div className="flex gap-2">
              <Input
                type="password"
                placeholder="**********"
                value={getApiKeyValue()}
                onChange={(value) => {
                  const apiKeyVar = findKeyAndValue("api_key");
                  if (!apiKeyVar || !selectedAIProvider) return;

                  onSetSelectedAIProvider({
                    ...selectedAIProvider,
                    variables: {
                      ...selectedAIProvider.variables,
                      [apiKeyVar.key]:
                        typeof value === "string" ? value : value.target.value,
                    },
                  });
                }}
                onKeyDown={(e) => {
                  const apiKeyVar = findKeyAndValue("api_key");
                  if (!apiKeyVar || !selectedAIProvider) return;

                  onSetSelectedAIProvider({
                    ...selectedAIProvider,
                    variables: {
                      ...selectedAIProvider.variables,
                      [apiKeyVar.key]: (e.target as HTMLInputElement).value,
                    },
                  });
                }}
                disabled={false}
                className="flex-1 h-11 border-1 border-input/50 focus:border-primary/50 transition-colors"
              />
              {isApiKeyEmpty() ? (
                <Button
                  onClick={() => {
                    const apiKeyVar = findKeyAndValue("api_key");
                    if (!apiKeyVar || !selectedAIProvider || isApiKeyEmpty())
                      return;

                    onSetSelectedAIProvider({
                      ...selectedAIProvider,
                      variables: {
                        ...selectedAIProvider.variables,
                        [apiKeyVar.key]: getApiKeyValue(),
                      },
                    });
                  }}
                  disabled={isApiKeyEmpty()}
                  size="icon"
                  className="shrink-0 h-11 w-11"
                  title="Submit API Key"
                >
                  <KeyIcon className="h-4 w-4" />
                </Button>
              ) : (
                <Button
                  onClick={() => {
                    const apiKeyVar = findKeyAndValue("api_key");
                    if (!apiKeyVar || !selectedAIProvider) return;

                    onSetSelectedAIProvider({
                      ...selectedAIProvider,
                      variables: {
                        ...selectedAIProvider.variables,
                        [apiKeyVar.key]: "",
                      },
                    });
                  }}
                  size="icon"
                  variant="destructive"
                  className="shrink-0 h-11 w-11"
                  title="Remove API Key"
                >
                  <TrashIcon className="h-4 w-4" />
                </Button>
              )}
            </div>
          </div>
        </div>
      ) : null}

      <div className="space-y-4 mt-2">
        {variables
          .filter(
            (variable) => variable.key !== findKeyAndValue("api_key")?.key
          )
          .map((variable) => {
            const getVariableValue = () => {
              if (!variable?.key || !selectedAIProvider?.variables) return "";
              return selectedAIProvider.variables[variable.key] || "";
            };

            return (
              <div className="space-y-1" key={variable?.key}>
                <Header
                  title={variable?.value || ""}
                  description={`add your preferred ${variable?.key?.replace(
                    /_/g,
                    " "
                  )} for ${
                    allAiProviders?.find(
                      (p) => p?.id === selectedAIProvider?.provider
                    )?.isCustom
                      ? "Custom Provider"
                      : getProviderDisplayName(selectedAIProvider?.provider)
                  }`}
                />
                <TextInput
                  placeholder={`Enter ${
                    allAiProviders?.find(
                      (p) => p?.id === selectedAIProvider?.provider
                    )?.isCustom
                      ? "Custom Provider"
                      : getProviderDisplayName(selectedAIProvider?.provider)
                  } ${variable?.key?.replace(/_/g, " ") || "value"}`}
                  value={getVariableValue()}
                  onChange={(value) => {
                    if (!variable?.key || !selectedAIProvider) return;

                    onSetSelectedAIProvider({
                      ...selectedAIProvider,
                      variables: {
                        ...selectedAIProvider.variables,
                        [variable.key]: value,
                      },
                    });
                  }}
                />
              </div>
            );
          })}
      </div>
    </div>
  );
};
