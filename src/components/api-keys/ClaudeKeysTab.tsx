import { createEffect, createMemo, createSignal, For, Show, splitProps } from "solid-js";
import { useI18n } from "../../i18n";
import {
  getClaudeApiKeys,
  setClaudeApiKeys,
  validateClaudeApiKeys,
  cleanupClaudeApiKeys,
} from "../../lib/tauri";
import { appStore } from "../../stores/app";
import { toastStore } from "../../stores/toast";
import { Button } from "../ui";

import type { ClaudeApiKey, ClaudeKeyHealth } from "../../lib/tauri";

interface ClaudeKeysTabProps {
  loading: () => boolean;
  setLoading: (value: boolean) => void;
  setShowAddForm: (value: boolean) => void;
  showAddForm: () => boolean;
}

export function ClaudeKeysTab(props: ClaudeKeysTabProps) {
  const [local] = splitProps(props, ["showAddForm", "setShowAddForm", "loading", "setLoading"]);
  const { t } = useI18n();
  const { proxyStatus } = appStore;
  const [claudeKeys, setClaudeKeys] = createSignal<ClaudeApiKey[]>([]);
  const [validationResults, setValidationResults] = createSignal<ClaudeKeyHealth[] | null>(null);
  const [validating, setValidating] = createSignal(false);
  const [newClaudeKey, setNewClaudeKey] = createSignal<ClaudeApiKey>({
    apiKey: "",
  });

  const maskApiKey = (key: string) => {
    if (key.length <= 8) {
      return "****";
    }
    return `${key.slice(0, 4)}...${key.slice(-4)}`;
  };

  const loadKeys = async () => {
    if (!proxyStatus().running) {
      return;
    }

    local.setLoading(true);
    try {
      const claude = await getClaudeApiKeys();
      setClaudeKeys(claude);
    } catch (error) {
      console.error("Failed to load Claude API keys:", error);
      toastStore.error(t("apiKeys.toasts.failedToLoadApiKeys"), String(error));
    } finally {
      local.setLoading(false);
    }
  };

  createEffect(() => {
    if (proxyStatus().running) {
      void loadKeys();
    }
  });

  const handleAddClaudeKey = async () => {
    const key = newClaudeKey();
    if (!key.apiKey.trim()) {
      toastStore.error(t("apiKeys.toasts.apiKeyRequired"));
      return;
    }

    local.setLoading(true);
    try {
      const updated = [...claudeKeys(), key];
      await setClaudeApiKeys(updated);
      setClaudeKeys(updated);
      setNewClaudeKey({ apiKey: "" });
      local.setShowAddForm(false);
      toastStore.success(t("apiKeys.toasts.apiKeyAdded", { provider: "Claude" }));
    } catch (error) {
      toastStore.error(t("apiKeys.toasts.failedToAddKey"), String(error));
    } finally {
      local.setLoading(false);
    }
  };

  const handleDeleteClaudeKey = async (index: number) => {
    local.setLoading(true);
    try {
      const updated = claudeKeys().filter((_, i) => i !== index);
      await setClaudeApiKeys(updated);
      setClaudeKeys(updated);
      toastStore.success(t("apiKeys.toasts.apiKeyDeleted", { provider: "Claude" }));
      setValidationResults(null); // Clear validation after change
    } catch (error) {
      toastStore.error(t("apiKeys.toasts.failedToDeleteKey"), String(error));
    } finally {
      local.setLoading(false);
    }
  };

  const handleValidateKeys = async () => {
    setValidating(true);
    setValidationResults(null);
    try {
      const results = await validateClaudeApiKeys();
      setValidationResults(results);

      const bad = results.filter((r) => r.status === "invalid" || r.status === "low_balance");
      if (bad.length > 0) {
        toastStore.warning(
          `Found ${bad.length} problematic key(s). Use "Cleanup" to remove them.`,
        );
      } else if (results.every((r) => r.status === "valid")) {
        toastStore.success("All Claude API keys are valid!");
      }
    } catch (error) {
      toastStore.error("Validation failed", String(error));
    } finally {
      setValidating(false);
    }
  };

  const handleCleanupKeys = async () => {
    local.setLoading(true);
    try {
      const results = await cleanupClaudeApiKeys();
      setValidationResults(results);
      // Reload keys
      const claude = await getClaudeApiKeys();
      setClaudeKeys(claude);

      const removed = results.filter((r) => r.status === "invalid" || r.status === "low_balance");
      if (removed.length > 0) {
        toastStore.success(`Removed ${removed.length} problematic key(s)`);
      } else {
        toastStore.success("No problematic keys to remove");
      }
    } catch (error) {
      toastStore.error("Cleanup failed", String(error));
    } finally {
      local.setLoading(false);
    }
  };

  const showEmptyState = createMemo(
    () =>
      proxyStatus().running &&
      !local.loading() &&
      claudeKeys().length === 0 &&
      !local.showAddForm(),
  );

  return (
    <div class="space-y-4">
      <Show when={claudeKeys().length > 0}>
        <div class="flex items-center gap-2">
          <Button
            disabled={validating() || !proxyStatus().running}
            onClick={handleValidateKeys}
            size="sm"
            variant="secondary"
          >
            <svg
              class="mr-1 h-4 w-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
              />
            </svg>
            {validating() ? "Validating..." : "Validate All"}
          </Button>
          <Button
            disabled={local.loading() || !proxyStatus().running}
            onClick={handleCleanupKeys}
            size="sm"
            variant="danger"
          >
            <svg
              class="mr-1 h-4 w-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
              />
            </svg>
            Cleanup Dead Keys
          </Button>
        </div>

        <Show when={validationResults()}>
          <div class="space-y-1">
            <For each={validationResults()}>
              {(result) => (
                <div
                  class={`flex items-center justify-between rounded-lg border p-2 text-sm ${
                    result.status === "valid"
                      ? "border-green-200 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-900/20 dark:text-green-400"
                      : result.status === "low_balance"
                        ? "border-yellow-200 bg-yellow-50 text-yellow-700 dark:border-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400"
                        : "border-red-200 bg-red-50 text-red-700 dark:border-red-800 dark:bg-red-900/20 dark:text-red-400"
                  }`}
                >
                  <div class="min-w-0 flex-1">
                    <code class="font-mono text-xs">{result.keyPrefix}</code>
                    <p class="text-xs opacity-75">{result.message}</p>
                  </div>
                  <span
                    class={`ml-2 rounded-full px-2 py-0.5 text-xs font-medium ${
                      result.status === "valid"
                        ? "bg-green-200 text-green-800 dark:bg-green-800 dark:text-green-200"
                        : result.status === "low_balance"
                          ? "bg-yellow-200 text-yellow-800 dark:bg-yellow-800 dark:text-yellow-200"
                          : "bg-red-200 text-red-800 dark:bg-red-800 dark:text-red-200"
                    }`}
                  >
                    {result.status === "valid"
                      ? "OK"
                      : result.status === "low_balance"
                        ? "No Balance"
                        : "Invalid"}
                  </span>
                </div>
              )}
            </For>
          </div>
        </Show>

        <div class="space-y-2">
          <For each={claudeKeys()}>
            {(key, index) => (
              <div class="flex items-center justify-between rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-800/50">
                <div class="min-w-0 flex-1">
                  <code class="font-mono text-sm text-gray-700 dark:text-gray-300">
                    {maskApiKey(key.apiKey)}
                  </code>
                  <Show when={key.baseUrl}>
                    <p class="mt-0.5 truncate text-xs text-gray-500 dark:text-gray-400">
                      {key.baseUrl}
                    </p>
                  </Show>
                </div>
                <Button onClick={() => handleDeleteClaudeKey(index())} size="sm" variant="ghost">
                  <svg
                    class="h-4 w-4 text-red-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                    />
                  </svg>
                </Button>
              </div>
            )}
          </For>
        </div>
      </Show>

      <Show when={local.showAddForm()}>
        <div class="space-y-3 rounded-xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-700 dark:bg-gray-800/50">
          <label class="block">
            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
              {t("apiKeys.labels.apiKeyRequired")}
            </span>
            <input
              class="mt-1 block w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-transparent focus:ring-2 focus:ring-brand-500 dark:border-gray-600 dark:bg-gray-900"
              onInput={(e) =>
                setNewClaudeKey({
                  ...newClaudeKey(),
                  apiKey: e.currentTarget.value,
                })
              }
              placeholder={t("apiKeys.placeholders.claudeApiKey")}
              type="password"
              value={newClaudeKey().apiKey}
            />
          </label>
          <label class="block">
            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
              {t("apiKeys.labels.baseUrlOptional")}
            </span>
            <input
              class="mt-1 block w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-transparent focus:ring-2 focus:ring-brand-500 dark:border-gray-600 dark:bg-gray-900"
              onInput={(e) =>
                setNewClaudeKey({
                  ...newClaudeKey(),
                  baseUrl: e.currentTarget.value || undefined,
                })
              }
              placeholder={t("apiKeys.placeholders.claudeBaseUrl")}
              type="text"
              value={newClaudeKey().baseUrl || ""}
            />
          </label>
          <label class="block">
            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
              {t("apiKeys.labels.prefixOptional")}
            </span>
            <input
              class="mt-1 block w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-transparent focus:ring-2 focus:ring-brand-500 dark:border-gray-600 dark:bg-gray-900"
              onInput={(e) =>
                setNewClaudeKey({
                  ...newClaudeKey(),
                  prefix: e.currentTarget.value || undefined,
                })
              }
              placeholder={t("apiKeys.placeholders.claudePrefix")}
              type="text"
              value={newClaudeKey().prefix || ""}
            />
          </label>
          <div class="flex gap-2 pt-2">
            <Button
              disabled={local.loading()}
              onClick={handleAddClaudeKey}
              size="sm"
              variant="primary"
            >
              {t("apiKeys.actions.addKey")}
            </Button>
            <Button onClick={() => local.setShowAddForm(false)} size="sm" variant="ghost">
              {t("common.cancel")}
            </Button>
          </div>
        </div>
      </Show>

      <Show when={!local.showAddForm()}>
        <Button
          class="w-full"
          disabled={!proxyStatus().running}
          onClick={() => local.setShowAddForm(true)}
          variant="secondary"
        >
          <svg class="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              d="M12 4v16m8-8H4"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
            />
          </svg>
          {t("apiKeys.actions.addClaudeApiKey")}
        </Button>
      </Show>

      <Show when={showEmptyState()}>
        <div class="py-8 text-center text-gray-500 dark:text-gray-400">
          <svg
            class="mx-auto mb-3 h-12 w-12 text-gray-300 dark:text-gray-600"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
            />
          </svg>
          <p class="text-sm">{t("apiKeys.noApiKeysConfiguredYet")}</p>
          <p class="mt-1 text-xs">{t("apiKeys.addFirstKeyHint")}</p>
        </div>
      </Show>
    </div>
  );
}
