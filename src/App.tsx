import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";
import {
  AlertTriangle,
  ChevronRight,
  CheckCircle2,
  Laptop,
  Loader2,
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
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen w-full max-w-[460px] flex-col gap-5 px-5 py-6">
        <header className="flex items-center justify-between gap-3">
          <div>
            <p className="text-sm text-muted-foreground">Lovstudio</p>
            <h1 className="font-serif text-2xl leading-tight">Mac Toolkits</h1>
          </div>
          <StatusBadge busy={busy} state={state} />
        </header>

        <section className="rounded-lg border border-border bg-card p-4 text-card-foreground">
          <div className="flex items-center justify-between gap-4">
            <div className="flex min-w-0 items-center gap-3">
              <IconBox active={Boolean(state?.protect_all_apps)}>
                <Laptop className="h-5 w-5" aria-hidden="true" />
              </IconBox>
              <div className="min-w-0">
                <h2 className="text-base font-medium leading-tight">Protect All Apps</h2>
                <p className="mt-1 truncate text-sm text-muted-foreground">
                  {protectAllLabel(state)}
                </p>
              </div>
            </div>
            <Switch
              checked={state?.protect_all_apps ?? false}
              disabled={busy}
              aria-label="Protect All Apps"
              onCheckedChange={(checked) => protectAllMutation.mutate(checked)}
            />
          </div>
        </section>

        <section className="rounded-lg border border-border bg-card p-4 text-card-foreground">
          <div className="mb-4 flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3">
              <IconBox active={Boolean(state?.whitelist.length)}>
                <ShieldCheck className="h-5 w-5" aria-hidden="true" />
              </IconBox>
              <div className="min-w-0">
                <h2 className="text-base font-medium leading-tight">Privileged Apps</h2>
                <p className="mt-1 truncate text-sm text-muted-foreground">
                  {privilegedAppsLabel(state)}
                </p>
              </div>
            </div>
          </div>

          <RunningAppList
            apps={state?.running_apps ?? []}
            protectAll={state?.protect_all_apps ?? false}
            disabled={busy}
            onChange={(name, enabled) => whitelistMutation.mutate({ name, enabled })}
          />
        </section>

        <div className="flex items-center justify-between gap-3">
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
            Refresh
          </Button>
          <p className="text-sm text-muted-foreground">{busy ? "Working" : "Ready"}</p>
        </div>

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

function StatusBadge({ busy, state }: { busy: boolean; state: AppProtectionState | undefined }) {
  if (busy) {
    return (
      <span className="inline-flex items-center gap-2 rounded-md border border-border bg-muted px-2.5 py-1 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
        Working
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-2 rounded-md border border-border bg-card px-2.5 py-1 text-sm text-muted-foreground">
      <CheckCircle2
        className={state?.effective_lid_protection ? "h-4 w-4 text-primary" : "h-4 w-4"}
        aria-hidden="true"
      />
      {state?.effective_lid_protection ? "Protected" : "Off"}
    </span>
  );
}

function IconBox({ active, children }: { active: boolean; children: ReactNode }) {
  return (
    <div
      className={
        active
          ? "flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground"
          : "flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"
      }
    >
      {children}
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
      <div className="rounded-md border border-border bg-background/60 p-3 text-sm text-muted-foreground">
        No running apps detected
      </div>
    );
  }

  const primaryApps = apps.filter((app) => app.kind !== "system" || app.whitelisted);
  const secondaryApps = apps.filter((app) => app.kind === "system" && !app.whitelisted);

  return (
    <div className="flex flex-col gap-2">
      {primaryApps.length > 0 ? (
        <div className="flex max-h-64 flex-col gap-1 overflow-auto rounded-md border border-border bg-background/60 p-2">
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
        <div className="rounded-md border border-border bg-background/60 p-3 text-sm text-muted-foreground">
          No user apps or CLI tools detected
        </div>
      )}

      {secondaryApps.length > 0 ? (
        <details className="group rounded-md border border-border bg-background/60">
          <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-3 py-2 text-sm text-muted-foreground [&::-webkit-details-marker]:hidden">
            <span className="inline-flex min-w-0 items-center gap-2">
              <ChevronRight
                className="h-4 w-4 shrink-0 transition-transform group-open:rotate-90"
                aria-hidden="true"
              />
              <span className="truncate">{secondaryApps.length} background processes</span>
            </span>
          </summary>
          <div className="flex max-h-48 flex-col gap-1 overflow-auto border-t border-border p-2">
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
    <div className="flex items-center justify-between gap-3 rounded-md bg-muted px-2 py-2 text-sm">
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
  return state.protect_all_apps ? "Global protection on" : "Global protection off";
}

function privilegedAppsLabel(state: AppProtectionState | undefined) {
  if (!state) {
    return "Loading";
  }
  const runningCount = state.running_apps.length;
  if (state.whitelist.length === 0) {
    return `${runningCount} running apps detected`;
  }
  if (state.whitelist.length === 1) {
    return `1 privileged, ${runningCount} running detected`;
  }
  return `${state.whitelist.length} privileged, ${runningCount} running detected`;
}

function appStatusLabel(app: RunningApp, protectAll: boolean) {
  if (app.whitelisted) {
    return "Always protected";
  }
  if (protectAll) {
    return `${runningKindLabel(app.kind)}, covered by global`;
  }
  if (app.count === 1) {
    return runningKindLabel(app.kind);
  }
  return `${app.count} ${runningKindLabel(app.kind).toLowerCase()} processes`;
}

function runningKindLabel(kind: RunningApp["kind"]) {
  if (kind === "cli") {
    return "CLI running";
  }
  if (kind === "system") {
    return "System background";
  }
  return "App running";
}
