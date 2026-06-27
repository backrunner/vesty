import { onScopeDispose, readonly, shallowRef, type ShallowRef } from "vue";
import {
  createSnapshotStore,
  type PluginSnapshot,
  type SnapshotStoreOptions,
  type VestyBridge,
  type VestySnapshotStore
} from "@vesty/plugin-ui";

export interface VestySnapshotComposable<TSnapshot extends PluginSnapshot = PluginSnapshot> {
  snapshot: Readonly<ShallowRef<TSnapshot | undefined>>;
  store: VestySnapshotStore<TSnapshot>;
  refresh(): Promise<TSnapshot>;
}

export interface VestyParamEditActions {
  begin(gestureId?: string): Promise<void>;
  perform(normalized: number, gestureId?: string): Promise<void>;
  end(gestureId?: string): Promise<void>;
  set(normalized: number, gestureId?: string): Promise<void>;
  format(normalized: number): Promise<string>;
  parse(text: string): Promise<number>;
}

export function useVestySnapshot<TSnapshot extends PluginSnapshot = PluginSnapshot>(
  bridge: VestyBridge,
  initialSnapshot?: TSnapshot,
  options: SnapshotStoreOptions = {}
): VestySnapshotComposable<TSnapshot> {
  const store = createSnapshotStore(bridge, initialSnapshot, options);
  const snapshot = shallowRef<TSnapshot | undefined>(store.getSnapshot());
  const unsubscribe = store.subscribe((next) => {
    snapshot.value = next;
  });

  onScopeDispose(() => {
    unsubscribe();
    store.dispose();
  });

  if (!snapshot.value) void store.refresh().catch(() => undefined);

  return {
    snapshot: readonly(snapshot) as Readonly<ShallowRef<TSnapshot | undefined>>,
    store,
    refresh: store.refresh
  };
}

export function useVestyParamEdit(id: string, bridge: VestyBridge): VestyParamEditActions {
  return {
    begin: (gestureId) => bridge.beginParamEdit(id, gestureId),
    perform: (normalized, gestureId) => bridge.performParamEdit(id, normalized, gestureId),
    end: (gestureId) => bridge.endParamEdit(id, gestureId),
    set: (normalized, gestureId) => bridge.setParam(id, normalized, gestureId),
    format: (normalized) => bridge.formatParam(id, normalized),
    parse: (text) => bridge.parseParam(id, text)
  };
}
