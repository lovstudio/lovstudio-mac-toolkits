import { invoke } from "@tauri-apps/api/core";

export type RunningApp = {
  name: string;
  count: number;
  kind: "cli" | "app" | "system";
  whitelisted: boolean;
  protected: boolean;
};

export type AppProtectionState = {
  protect_all_apps: boolean;
  effective_lid_protection: boolean;
  protected_count: number;
  whitelist: string[];
  running_apps: RunningApp[];
};

export function getAppProtection() {
  return invoke<AppProtectionState>("get_app_protection");
}

export function setProtectAllApps(enabled: boolean) {
  return invoke<AppProtectionState>("set_protect_all_apps", { enabled });
}

export function setAppWhitelist(name: string, enabled: boolean) {
  return invoke<AppProtectionState>("set_app_whitelist", { name, enabled });
}

export function openSettingsWindow() {
  return invoke<void>("open_settings_window");
}
