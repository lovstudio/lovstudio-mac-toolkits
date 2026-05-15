import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";
import {
  AlertTriangle,
  ChevronRight,
  CheckCircle2,
  Layers2,
  Loader2,
  Moon,
  PanelTop,
  Plug,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  getAppProtection,
  type AppProtectionState,
  type RunningApp,
  setAppWhitelist,
  setProtectAllApps,
} from "@/lib/tauri";

const appProtectionQueryKey = ["app-protection"] as const;

export function App() {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: appProtectionQueryKey,
    queryFn: getAppProtection,
  });

  const protectAllMutation = useMutation({
    mutationFn: (enabled: boolean) => setProtectAllApps(enabled),
    onSuccess: (state) => {
      queryClient.setQueryData(appProtectionQueryKey, state);
    },
  });
  const whitelistMutation = useMutation({
    mutationFn: ({ name, enabled }: { name: string; enabled: boolean }) =>
      setAppWhitelist(name, enabled),
    onSuccess: (state) => {
      queryClient.setQueryData(appProtectionQueryKey, state);
    },
  });

  const state = (protectAllMutation.data ?? whitelistMutation.data ?? query.data) as
    | AppProtectionState
    | undefined;
  const busy = query.isLoading || protectAllMutation.isPending || whitelistMutation.isPending;
  const error = protectAllMutation.error ?? whitelistMutation.error ?? query.error;

  return (
    <main className="min-h-screen overflow-hidden bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 bg-[linear-gradient(90deg,rgba(24,24,24,0.035)_1px,transparent_1px),linear-gradient(180deg,rgba(24,24,24,0.028)_1px,transparent_1px)] bg-[size:36px_36px]" />
      <div className="relative mx-auto flex min-h-screen w-full max-w-[500px] flex-col gap-4 px-5 py-5">
        <header className="flex items-start justify-between gap-4 border-b border-border pb-4">
          <div className="flex min-w-0 items-start gap-3">
            <img src="/logo.svg" width="34" height="34" alt="" className="mt-1 shrink-0" />
            <div className="min-w-0">
              <p className="text-xs font-medium text-primary">Lovstudio.ai / 手工川工作室</p>
              <h1 className="font-serif text-2xl leading-tight">Mac Menu Manager</h1>
              <p className="mt-1 truncate text-sm text-muted-foreground">
                Pluggable menu bar modules
              </p>
            </div>
          </div>
          <StatusBadge busy={busy} state={state} />
        </header>

        <ModuleDock />

        <section className="rounded-lg border border-border bg-card/95 p-4 text-card-foreground shadow-sm">
          <div className="flex items-start justify-between gap-4">
            <div className="flex min-w-0 items-start gap-3">
              <IconBox active={state?.effective_lid_protection ?? false}>
                <Moon className="h-5 w-5" aria-hidden="true" />
              </IconBox>
              <div className="min-w-0">
                <p className="text-xs font-medium uppercase text-muted-foreground">Module 01</p>
                <h2 className="text-lg font-semibold leading-tight">Lid Sleep Guard</h2>
                <p className="mt-1 text-sm text-muted-foreground">防盒盖休眠</p>
              </div>
            </div>
            <ModuleState state={state} busy={busy} />
          </div>

          <div className="mt-4 grid gap-2">
            <ControlRow
              icon={<PanelTop className="h-4 w-4" aria-hidden="true" />}
              title="全局模式"
              label={protectAllLabel(state)}
              checked={state?.protect_all_apps ?? false}
              disabled={busy}
              ariaLabel="Toggle global lid sleep guard"
              onCheckedChange={(checked) => protectAllMutation.mutate(checked)}
            />
            <div className="rounded-md border border-border bg-background/70 px-3 py-3">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div className="flex min-w-0 items-center gap-2">
                  <ShieldCheck className="h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium">特权应用</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {privilegedAppsLabel(state)}
                    </div>
                  </div>
                </div>
                <span className="shrink-0 rounded-md bg-secondary px-2 py-1 text-xs text-muted-foreground">
                  Always-on
                </span>
              </div>
              <RunningAppList
                apps={state?.running_apps ?? []}
                protectAll={state?.protect_all_apps ?? false}
                disabled={busy}
                onChange={(name, enabled) => whitelistMutation.mutate({ name, enabled })}
              />
            </div>
          </div>
        </section>

        <footer className="flex items-center justify-between gap-3">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={busy}
            onClick={() => {
              void query.refetch();
            }}
          >
            <RefreshCw className={busy ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
            刷新
          </Button>
          <p className="text-sm text-muted-foreground">{busy ? "同步中" : "Ready"}</p>
        </footer>

        {error ? (
          <div className="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
            <p>{error instanceof Error ? error.message : String(error)}</p>
          </div>
        ) : null}
      </div>
    </main>
  );
}

function ModuleDock() {
  return (
    <nav className="grid grid-cols-[1fr_auto] gap-2" aria-label="Modules">
      <div className="flex items-center gap-3 rounded-lg border border-primary/35 bg-card/90 px-3 py-2 shadow-sm">
        <Moon className="h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">Lid Sleep Guard</div>
          <div className="text-xs text-primary">Active module</div>
        </div>
      </div>
      <div className="flex items-center gap-2 rounded-lg border border-dashed border-border bg-background/60 px-3 py-2 text-muted-foreground">
        <Plug className="h-4 w-4" aria-hidden="true" />
        <Layers2 className="h-4 w-4" aria-hidden="true" />
      </div>
    </nav>
  );
}

function StatusBadge({ busy, state }: { busy: boolean; state: AppProtectionState | undefined }) {
  if (busy) {
    return (
      <span className="inline-flex shrink-0 items-center gap-2 rounded-md border border-border bg-muted px-2.5 py-1 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
        Sync
      </span>
    );
  }

  return (
    <span className="inline-flex shrink-0 items-center gap-2 rounded-md border border-border bg-card px-2.5 py-1 text-sm text-muted-foreground">
      <CheckCircle2
        className={state?.effective_lid_protection ? "h-4 w-4 text-primary" : "h-4 w-4"}
        aria-hidden="true"
      />
      {state?.effective_lid_protection ? "Awake" : "Idle"}
    </span>
  );
}

function ModuleState({ busy, state }: { busy: boolean; state: AppProtectionState | undefined }) {
  if (busy) {
    return <span className="shrink-0 text-sm text-muted-foreground">Sync</span>;
  }
  return (
    <span className="shrink-0 rounded-md bg-secondary px-2.5 py-1 text-sm text-secondary-foreground">
      {state?.effective_lid_protection ? "Enabled" : "Paused"}
    </span>
  );
}

function IconBox({ active, children }: { active: boolean; children: ReactNode }) {
  return (
    <div
      className={
        active
          ? "flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground shadow-sm"
          : "flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"
      }
    >
      {children}
    </div>
  );
}

function ControlRow({
  icon,
  title,
  label,
  checked,
  disabled,
  ariaLabel,
  onCheckedChange,
}: {
  icon: ReactNode;
  title: string;
  label: string;
  checked: boolean;
  disabled: boolean;
  ariaLabel: string;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-border bg-background/70 px-3 py-3">
      <div className="flex min-w-0 items-center gap-2">
        <span className="text-primary">{icon}</span>
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{title}</div>
          <div className="truncate text-xs text-muted-foreground">{label}</div>
        </div>
      </div>
      <Switch
        checked={checked}
        disabled={disabled}
        aria-label={ariaLabel}
        onCheckedChange={onCheckedChange}
      />
    </div>
  );
}

function RunningAppList({
  apps,
  protectAll,
  disabled,
  onChange,
}: {
  apps: RunningApp[];
  protectAll: boolean;
  disabled: boolean;
  onChange: (name: string, enabled: boolean) => void;
}) {
  if (apps.length === 0) {
    return (
      <div className="rounded-md border border-border bg-card/80 p-3 text-sm text-muted-foreground">
        未检测到运行项
      </div>
    );
  }

  const primaryApps = apps.filter((app) => app.kind !== "system" || app.whitelisted);
  const secondaryApps = apps.filter((app) => app.kind === "system" && !app.whitelisted);

  return (
    <div className="flex flex-col gap-2">
      {primaryApps.length > 0 ? (
        <div className="flex max-h-56 flex-col gap-1 overflow-auto">
          {primaryApps.map((app) => (
            <RunningAppRow
              key={app.name}
              app={app}
              protectAll={protectAll}
              disabled={disabled}
              onChange={onChange}
            />
          ))}
        </div>
      ) : (
        <div className="rounded-md border border-border bg-card/80 p-3 text-sm text-muted-foreground">
          未检测到用户 App 或 CLI 工具
        </div>
      )}

      {secondaryApps.length > 0 ? (
        <details className="group rounded-md border border-border bg-card/70">
          <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-3 py-2 text-sm text-muted-foreground [&::-webkit-details-marker]:hidden">
            <span className="inline-flex min-w-0 items-center gap-2">
              <ChevronRight
                className="h-4 w-4 shrink-0 transition-transform group-open:rotate-90"
                aria-hidden="true"
              />
              <span className="truncate">{secondaryApps.length} 个后台进程</span>
            </span>
          </summary>
          <div className="flex max-h-44 flex-col gap-1 overflow-auto border-t border-border p-2">
            {secondaryApps.map((app) => (
              <RunningAppRow
                key={app.name}
                app={app}
                protectAll={protectAll}
                disabled={disabled}
                onChange={onChange}
              />
            ))}
          </div>
        </details>
      ) : null}
    </div>
  );
}

function RunningAppRow({
  app,
  protectAll,
  disabled,
  onChange,
}: {
  app: RunningApp;
  protectAll: boolean;
  disabled: boolean;
  onChange: (name: string, enabled: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md bg-muted px-2.5 py-2 text-sm">
      <div className="min-w-0">
        <div className="truncate font-medium">{app.name}</div>
        <div className="text-xs text-muted-foreground">
          {appStatusLabel(app, protectAll)}
        </div>
      </div>
      <Switch
        checked={app.whitelisted}
        disabled={disabled}
        aria-label={`Protect ${app.name}`}
        onCheckedChange={(checked) => onChange(app.name, checked)}
      />
    </div>
  );
}

function protectAllLabel(state: AppProtectionState | undefined) {
  if (!state) {
    return "Loading";
  }
  return state.protect_all_apps ? "所有运行项已纳入" : "仅保护特权应用";
}

function privilegedAppsLabel(state: AppProtectionState | undefined) {
  if (!state) {
    return "Loading";
  }
  const runningCount = state.running_apps.length;
  if (state.whitelist.length === 0) {
    return `${runningCount} 个运行项`;
  }
  return `${state.whitelist.length} 个特权项 / ${runningCount} 个运行项`;
}

function appStatusLabel(app: RunningApp, protectAll: boolean) {
  if (app.whitelisted) {
    return "特权保护";
  }
  if (protectAll) {
    return `${runningKindLabel(app.kind)} / 全局保护`;
  }
  if (app.count === 1) {
    return runningKindLabel(app.kind);
  }
  return `${app.count} 个${runningKindLabel(app.kind)}`;
}

function runningKindLabel(kind: RunningApp["kind"]) {
  if (kind === "cli") {
    return "CLI 进程";
  }
  if (kind === "system") {
    return "后台进程";
  }
  return "App 进程";
}
