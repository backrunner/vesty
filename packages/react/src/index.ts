import * as React from "react";
import {
  createSnapshotStore,
  type PluginSnapshot,
  type SnapshotStoreOptions,
  type VestyBridge,
  type VestySnapshotStore
} from "@vesty/plugin-ui";

export interface VestyBridgeProviderProps {
  bridge: VestyBridge;
  children?: React.ReactNode;
}

export interface VestyParamEditActions {
  begin(gestureId?: string): Promise<void>;
  perform(normalized: number, gestureId?: string): Promise<void>;
  end(gestureId?: string): Promise<void>;
  set(normalized: number, gestureId?: string): Promise<void>;
  format(normalized: number): Promise<string>;
  parse(text: string): Promise<number>;
}

export const VestyBridgeContext = React.createContext<VestyBridge | null>(null);

export function VestyBridgeProvider({
  bridge,
  children
}: VestyBridgeProviderProps): React.ReactElement {
  return React.createElement(VestyBridgeContext.Provider, { value: bridge }, children);
}

export function useVestyBridge(explicitBridge?: VestyBridge): VestyBridge {
  const contextBridge = React.useContext(VestyBridgeContext);
  const bridge = explicitBridge ?? contextBridge;
  if (!bridge) {
    throw new Error("Vesty bridge is unavailable. Wrap your UI in VestyBridgeProvider or pass a bridge.");
  }
  return bridge;
}

export function useVestySnapshotStore<TSnapshot extends PluginSnapshot = PluginSnapshot>(
  bridge?: VestyBridge,
  initialSnapshot?: TSnapshot,
  options: SnapshotStoreOptions = {}
): VestySnapshotStore<TSnapshot> {
  const resolvedBridge = useVestyBridge(bridge);
  const topic = options.topic ?? "state.changed";
  const refreshOnEvent = options.refreshOnEvent ?? true;
  const store = React.useMemo(
    () => createSnapshotStore(resolvedBridge, initialSnapshot, { topic, refreshOnEvent }),
    [resolvedBridge, initialSnapshot, topic, refreshOnEvent]
  );

  React.useEffect(() => () => store.dispose(), [store]);
  return store;
}

export function useVestySnapshot<TSnapshot extends PluginSnapshot = PluginSnapshot>(
  bridge?: VestyBridge,
  initialSnapshot?: TSnapshot,
  options: SnapshotStoreOptions = {}
): TSnapshot | undefined {
  const store = useVestySnapshotStore(bridge, initialSnapshot, options);
  const snapshot = React.useSyncExternalStore(
    (notify) => store.subscribe(() => notify()),
    () => store.getSnapshot(),
    () => store.getSnapshot()
  );

  React.useEffect(() => {
    if (!snapshot) void store.refresh().catch(() => undefined);
  }, [snapshot, store]);

  return snapshot;
}

export function useVestyParamEdit(id: string, bridge?: VestyBridge): VestyParamEditActions {
  const resolvedBridge = useVestyBridge(bridge);
  return React.useMemo(
    () => ({
      begin: (gestureId?: string) => resolvedBridge.beginParamEdit(id, gestureId),
      perform: (normalized: number, gestureId?: string) =>
        resolvedBridge.performParamEdit(id, normalized, gestureId),
      end: (gestureId?: string) => resolvedBridge.endParamEdit(id, gestureId),
      set: (normalized: number, gestureId?: string) => resolvedBridge.setParam(id, normalized, gestureId),
      format: (normalized: number) => resolvedBridge.formatParam(id, normalized),
      parse: (text: string) => resolvedBridge.parseParam(id, text)
    }),
    [resolvedBridge, id]
  );
}
